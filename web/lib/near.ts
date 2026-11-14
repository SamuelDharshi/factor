// Read-only NEAR access over plain JSON-RPC (no keys, no indexer). Writes go through the wallet.
import deployments from "./deployments.json";

// Public RPCs rate-limit browsers (a 429 shows up as a CORS error), so try several in order.
export const RPCS = ["https://test.rpc.fastnear.com", "https://rpc.testnet.near.org"];
export const RPC = RPCS[0];
export const EXPLORER = "https://testnet.nearblocks.io";
export const FACTOR: string = deployments.factor;
export const MUSDC: string = deployments.musdc;
export const DEFAULT_DAO: string = deployments.dao;
export const REGISTRY: string = (deployments as { registry?: string }).registry ?? "";
export const DEPLOYED = Boolean(FACTOR && MUSDC);

/** Fixed DEMO conversion used only to show an implied APR (face is in mUSDC, price in NEAR). */
export const DEMO_NEAR_PER_MUSDC = 0.02; // 1 NEAR = 50 mUSDC
export const MUSDC_DECIMALS = 6;
const YOCTO = BigInt("1000000000000000000000000");

export type Status = "Draft" | "Accepted" | "Paid" | "Settled";

export interface InvoiceView {
  id: string;
  owner: string;
  creator: string;
  payer_dao: string;
  vault: string;
  amount: string; // mUSDC base units
  due_ts: string; // ns
  created_ts: string;
  accepted_ts: string | null;
  paid_ts: string | null;
  settled_ts: string | null;
  paid_on_time: boolean | null;
  status: Status;
  pending: boolean;
  memo: string;
  is_overdue: boolean;
  is_listed: boolean;
  price: string | null; // yoctoNEAR
}

export interface PayerStatsView {
  dao: string;
  accepted: number;
  paid_on_time: number;
  paid_late: number;
  volume_paid: string;
  first_seen_ts: string;
  age_days: number;
  on_time_rate_pct: number;
  payer_score: number;
}

const inflight = new Map<string, { at: number; p: Promise<unknown> }>();
const CACHE_MS = 8000;

async function rpcOnce(url: string, method: string, params: unknown) {
  const res = await fetch(url, {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({ jsonrpc: "2.0", id: "1", method, params }),
  });
  if (!res.ok) throw new Error(`rpc ${res.status}`);
  const j = await res.json();
  if (j.error) throw new Error(j.error.data ?? j.error.message ?? "rpc error");
  if (j.result?.error) throw new Error(j.result.error);
  return j.result;
}

/** JSON-RPC with endpoint fallback, in-flight de-duplication and a short cache. */
async function rpc(method: string, params: unknown) {
  const key = method + JSON.stringify(params);
  const hit = inflight.get(key);
  if (hit && Date.now() - hit.at < CACHE_MS) return hit.p;
  const p = (async () => {
    let last: unknown;
    for (const url of RPCS) {
      try { return await rpcOnce(url, method, params); } catch (e) { last = e; }
    }
    throw last instanceof Error ? last : new Error("all RPC endpoints failed");
  })();
  inflight.set(key, { at: Date.now(), p });
  p.catch(() => inflight.delete(key));
  return p;
}

export async function view<T>(contract: string, method: string, args: object = {}): Promise<T> {
  const r = await rpc("query", {
    request_type: "call_function",
    finality: "optimistic",
    account_id: contract,
    method_name: method,
    args_base64: btoa(JSON.stringify(args)),
  });
  return JSON.parse(new TextDecoder().decode(new Uint8Array(r.result))) as T;
}

/** Real RPC view_access_key_list: how many keys does this account have? */
export async function accessKeyCount(account: string): Promise<number> {
  const r = await rpc("query", { request_type: "view_access_key_list", finality: "final", account_id: account });
  return (r.keys as unknown[]).length;
}

export async function ftBalance(account: string): Promise<bigint> {
  return BigInt(await view<string>(MUSDC, "ft_balance_of", { account_id: account }));
}

export const getInvoice = (id: string) => view<InvoiceView | null>(FACTOR, "invoice", { id });
export const getStats = (dao: string) => view<PayerStatsView>(FACTOR, "payer_stats", { dao });
export const getListings = () => view<InvoiceView[]>(FACTOR, "listings", { from_index: 0, limit: 100 });
export const getRecent = (limit = 30) => view<InvoiceView[]>(FACTOR, "recent_invoices", { limit });
export const getNextId = async () => (await view<{ next_id: string }>(FACTOR, "config")).next_id;

// ---- formatting / math
export function yoctoToNear(y: string | bigint, digits = 2): string {
  const v = BigInt(y);
  const whole = v / YOCTO;
  const frac = ((v % YOCTO) * BigInt(10 ** digits)) / YOCTO;
  return `${whole}.${frac.toString().padStart(digits, "0")}`;
}
export function nearToYocto(n: string): string {
  const [w, f = ""] = n.trim().split(".");
  if (!/^\d+$/.test(w || "0") || !/^\d*$/.test(f)) throw new Error("invalid NEAR amount");
  return (BigInt(w || "0") * YOCTO + BigInt((f + "0".repeat(24)).slice(0, 24))).toString();
}
export function fmtMusdc(base: string | bigint): string {
  const v = BigInt(base);
  const d = BigInt(10 ** MUSDC_DECIMALS);
  const cents = ((v % d) * BigInt(100)) / d;
  return `${(v / d).toLocaleString("en-US")}.${cents.toString().padStart(2, "0")}`;
}
export const musdcToBase = (n: string) => (BigInt(Math.round(parseFloat(n) * 100)) * BigInt(10 ** (MUSDC_DECIMALS - 2))).toString();
export const nsToDate = (ns: string) => new Date(Number(BigInt(ns) / BigInt(1_000_000)));
export const daysToDue = (inv: InvoiceView) => (Number(BigInt(inv.due_ts) / BigInt(1_000_000)) - Date.now()) / 86_400_000;

export function faceInNear(inv: InvoiceView): number {
  return (Number(BigInt(inv.amount)) / 10 ** MUSDC_DECIMALS) * DEMO_NEAR_PER_MUSDC;
}
export function discountPct(inv: InvoiceView): number | null {
  if (!inv.price) return null;
  const price = Number(BigInt(inv.price) / BigInt("1000000000000000000")) / 1e6;
  return (1 - price / faceInNear(inv)) * 100;
}
export function impliedApr(inv: InvoiceView): number | null {
  if (!inv.price) return null;
  const price = Number(BigInt(inv.price) / BigInt("1000000000000000000")) / 1e6;
  const days = Math.max(daysToDue(inv), 1);
  return ((faceInNear(inv) / price - 1) * 365) / days * 100;
}
export const scoreTone = (s: number) => (s >= 75 ? "green" : s >= 50 ? "amber" : "red");

/** Sputnik add_proposal args for a FunctionCall (FR-11); paste into the DAO UI / near-cli. */
export function sputnikProposal(description: string, receiver: string, method: string, args: object, depositYocto: string, tgas: number) {
  return {
    proposal: {
      description,
      kind: {
        FunctionCall: {
          receiver_id: receiver,
          actions: [
            { method_name: method, args: btoa(JSON.stringify(args)), deposit: depositYocto, gas: String(BigInt(tgas) * BigInt("1000000000000")) },
          ],
        },
      },
    },
  };
}


// ---------------------------------------------------------------- agent / robot registry
export type AgentKind = "Agent" | "Robot";
export interface AgentView {
  account_id: string;
  name: string;
  kind: AgentKind;
  description: string;
  capabilities: string[];
  endpoint: string;
  strategy: { min_score: number; min_apr_pct: number; max_price: string };
  registered_ts: string;
  active: boolean;
  storage_deposit: string;
}
export interface AgentInput {
  name: string;
  kind: AgentKind;
  description: string;
  capabilities: string[];
  endpoint: string;
  strategy: { min_score: number; min_apr_pct: number; max_price: string };
}

export const listAgents = (kind?: AgentKind) =>
  view<AgentView[]>(REGISTRY, "list_agents", { from_index: 0, limit: 100, kind: kind ?? null, active_only: true });
export const getAgent = (account_id: string) => view<AgentView | null>(REGISTRY, "get_agent", { account_id });
export const agentsCount = () => view<number>(REGISTRY, "agents_count");

export async function nearBalance(account: string): Promise<bigint> {
  const r = await rpc("query", { request_type: "view_account", finality: "final", account_id: account });
  return BigInt(r.amount);
}
/** How many FACTOR invoice NFTs this account holds. */
export async function invoicesHeld(account: string): Promise<number> {
  return Number(await view<string>(FACTOR, "nft_supply_for_owner", { account_id: account }));
}
export function timeAgo(ns: string): string {
  const s = Math.max(0, (Date.now() - Number(BigInt(ns) / BigInt(1_000_000))) / 1000);
  if (s < 90) return "just now";
  if (s < 5400) return `${Math.round(s / 60)} min ago`;
  if (s < 129_600) return `${Math.round(s / 3600)} h ago`;
  return `${Math.round(s / 86_400)} d ago`;
}
