"use client";
// Register (or manage) an agent / robot on the NEAR registry contract.
import { useRouter } from "next/navigation";
import { useEffect, useMemo, useState } from "react";
import AgentCard from "@/components/AgentCard";
import {
  REGISTRY, type AgentInput, type AgentKind, type AgentView, getAgent, nearToYocto, yoctoToNear,
} from "@/lib/near";
import { useWallet } from "@/lib/wallet";

const DEPOSIT = "100000000000000000000000"; // 0.1 NEAR; only the storage it uses is kept, the rest is refunded

export default function RegisterAgent() {
  const { account, connect, call, notify } = useWallet();
  const router = useRouter();
  const [existing, setExisting] = useState<AgentView | null | undefined>(undefined);
  const [kind, setKind] = useState<AgentKind>("Agent");
  const [name, setName] = useState("");
  const [description, setDescription] = useState("");
  const [capText, setCapText] = useState("buy-invoices, risk-scoring");
  const [endpoint, setEndpoint] = useState("");
  const [minScore, setMinScore] = useState(70);
  const [minApr, setMinApr] = useState(40);
  const [maxPrice, setMaxPrice] = useState("10");
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    if (!account || !REGISTRY) { setExisting(undefined); return; }
    getAgent(account).then((a) => {
      setExisting(a);
      if (a) {
        setKind(a.kind); setName(a.name); setDescription(a.description); setCapText(a.capabilities.join(", "));
        setEndpoint(a.endpoint); setMinScore(a.strategy.min_score); setMinApr(a.strategy.min_apr_pct);
        setMaxPrice(BigInt(a.strategy.max_price) === BigInt(0) ? "0" : yoctoToNear(a.strategy.max_price, 2));
      }
    }).catch(() => setExisting(null));
  }, [account]);

  const caps = useMemo(() => capText.split(",").map((c) => c.trim()).filter(Boolean), [capText]);
  const priceYocto = useMemo(() => { try { return nearToYocto(maxPrice || "0"); } catch { return null; } }, [maxPrice]);

  const errors: string[] = [];
  if (!name.trim() || name.length > 40) errors.push("Name is required (max 40 characters).");
  if (description.length > 280) errors.push("Description is max 280 characters.");
  if (caps.length > 8 || caps.some((c) => c.length > 24)) errors.push("Up to 8 capabilities, each up to 24 characters.");
  if (endpoint.length > 200) errors.push("Endpoint is max 200 characters.");
  if (priceYocto === null) errors.push("Max price must be a number of NEAR.");
  const valid = errors.length === 0;

  const input: AgentInput = {
    name: name.trim(), kind, description, capabilities: caps, endpoint,
    strategy: { min_score: minScore, min_apr_pct: minApr, max_price: priceYocto ?? "0" },
  };
  const preview: AgentView = {
    account_id: account ?? "your-account.testnet", ...input, registered_ts: String(BigInt(Date.now()) * BigInt(1_000_000)),
    active: true, storage_deposit: "0", name: input.name || "Your agent",
  };

  async function submit() {
    if (!account) return connect();
    setBusy(true);
    const hash = await call(REGISTRY, existing ? "update" : "register", { input }, DEPOSIT, 50, existing ? "Update listing" : "Register");
    setBusy(false);
    if (hash) setTimeout(() => router.push("/agents"), 1800);
  }
  async function pause(active: boolean) {
    await call(REGISTRY, "set_active", { active }, "0", 30, active ? "Resume" : "Pause");
  }
  async function remove() {
    if (!confirm("Remove your registration? Your storage deposit is refunded.")) return;
    const hash = await call(REGISTRY, "deregister", {}, "1", 30, "Deregister");
    if (hash) setTimeout(() => router.push("/agents"), 1800);
  }

  if (!REGISTRY) return <p className="muted">The registry is not deployed yet. Run <code>make deploy</code>.</p>;

  return (
    <div>
      <p className="eyebrow">register_agent</p>
      <h1 className="display mt-3 text-4xl font-semibold tracking-tight sm:text-5xl">{existing ? "Your agent" : "Register an agent"}</h1>
      <p className="mt-2 muted">Your connected account becomes the agent.</p>

      <div className="mt-10 grid gap-8 lg:grid-cols-[1.1fr_0.9fr]">
        <form className="card space-y-5 p-6" onSubmit={(e) => { e.preventDefault(); if (valid) submit(); }} noValidate>
          <fieldset>
            <legend className="mb-2 text-sm font-medium">Kind</legend>
            <div className="grid grid-cols-2 gap-3">
              {(["Agent", "Robot"] as AgentKind[]).map((k) => (
                <button key={k} type="button" onClick={() => setKind(k)} aria-pressed={kind === k}
                  className="min-h-[52px] rounded-xl border px-4 text-left transition-colors"
                  style={{ borderColor: kind === k ? "var(--ink)" : "var(--rule)", background: kind === k ? "rgba(0,236,151,0.14)" : "#fff" }}>
                  <span className="block font-semibold">{k === "Agent" ? "AI agent" : "Robot"}</span>
                  <span className="block text-xs muted">{k === "Agent" ? "software that trades" : "physical device / unit"}</span>
                </button>
              ))}
            </div>
          </fieldset>

          <label className="block text-sm font-medium">Name
            <input className="input mt-1.5 font-normal" value={name} maxLength={40} onChange={(e) => setName(e.target.value)} placeholder="Carol Capital" />
          </label>
          <label className="block text-sm font-medium">Description
            <textarea className="input mt-1.5 font-normal" rows={3} value={description} maxLength={280} onChange={(e) => setDescription(e.target.value)} placeholder="What does it do and when does it buy?" />
          </label>
          <label className="block text-sm font-medium">Capabilities <span className="font-normal muted">comma separated</span>
            <input className="input mt-1.5 font-normal" value={capText} onChange={(e) => setCapText(e.target.value)} />
          </label>
          <label className="block text-sm font-medium">Endpoint <span className="font-normal muted">optional</span>
            <input className="input mt-1.5 font-normal" value={endpoint} maxLength={200} onChange={(e) => setEndpoint(e.target.value)} placeholder="https://my-agent.example/mcp" />
          </label>

          <div className="grid gap-5 sm:grid-cols-3">
            <label className="block text-sm font-medium">Min payer score: <span className="mono" style={{ color: "var(--green)" }}>{minScore}</span>
              <input type="range" min={0} max={100} value={minScore} onChange={(e) => setMinScore(Number(e.target.value))} className="mt-3" />
            </label>
            <label className="block text-sm font-medium">Min implied APR: <span className="mono" style={{ color: "var(--green)" }}>{minApr}%</span>
              <input type="range" min={0} max={300} step={5} value={minApr} onChange={(e) => setMinApr(Number(e.target.value))} className="mt-3" />
            </label>
            <label className="block text-sm font-medium">Max price (NEAR)
              <input className="input mt-1.5 font-normal" inputMode="decimal" value={maxPrice} onChange={(e) => setMaxPrice(e.target.value)} />
            </label>
          </div>

          {errors.length > 0 && <ul className="space-y-1 text-sm" style={{ color: "var(--warn)" }} role="alert">{errors.map((e) => <li key={e}>{e}</li>)}</ul>}

          <div className="flex flex-wrap gap-3">
            <button className="btn" type="submit" disabled={busy || (!!account && !valid)}>
              {busy ? "Waiting for wallet…" : !account ? "Connect wallet to register" : existing ? "Update listing" : "Register on NEAR"}
            </button>
            {existing && (
              <>
                <button type="button" className="btn-ghost" onClick={() => pause(!existing.active)}>{existing.active ? "Pause" : "Resume"}</button>
                <button type="button" className="btn-ghost" onClick={remove}>Deregister</button>
              </>
            )}
          </div>
          <p className="text-xs muted">0.1 NEAR is attached; unused NEAR is refunded.</p>
        </form>

        <div>
          <p className="mb-3 font-mono text-xs uppercase tracking-widest muted">live preview</p>
          <AgentCard a={preview} preview />
          <p className="mt-4 text-xs muted">Strategies are public. The agent follows them itself.</p>
        </div>
      </div>
    </div>
  );
}
