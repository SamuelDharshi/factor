"use client";
import { useRouter } from "next/navigation";
import { useEffect, useState } from "react";
import {
  DEFAULT_DAO, FACTOR, MUSDC, getNextId, musdcToBase, sputnikProposal,
} from "@/lib/near";
import { useWallet } from "@/lib/wallet";

const CREATE_DEPOSIT = "100000000000000000000000"; // 0.1 NEAR; the contract keeps only its storage cost, the rest is refunded

export default function NewInvoice() {
  const { account, connect, call, notify } = useWallet();
  const router = useRouter();
  const [dao, setDao] = useState(DEFAULT_DAO);
  const [amount, setAmount] = useState("500");
  const [due, setDue] = useState(() => new Date(Date.now() + 25 * 86_400_000).toISOString().slice(0, 10));
  const [memo, setMemo] = useState("");
  const [nextId, setNextId] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [tab, setTab] = useState<"create" | "dao">("create");

  useEffect(() => { getNextId().then(setNextId).catch(() => setNextId(null)); }, []);

  const amountOk = /^\d+(\.\d{1,2})?$/.test(amount) && Number(amount) > 0;
  const dueMs = new Date(`${due}T23:59:59Z`).getTime();
  const valid = dao.includes(".") && amountOk && dueMs > Date.now() && memo.length <= 280;

  async function submit() {
    if (!account) return connect();
    setBusy(true);
    const id = await getNextId();
    const hash = await call(
      FACTOR, "create_invoice",
      { payer_dao: dao, amount: musdcToBase(amount), due_ts: String(BigInt(dueMs) * BigInt(1_000_000)), memo },
      CREATE_DEPOSIT, 100, "Create invoice",
    );
    setBusy(false);
    if (hash) router.push(`/invoice/${id}`);
  }

  async function copy(text: string, what: string) {
    try {
      await navigator.clipboard.writeText(text);
      notify({ kind: "ok", text: `${what} copied.` });
    } catch {
      notify({ kind: "err", text: "Clipboard blocked. Copy the JSON below." });
    }
  }

  const acceptJson = nextId
    ? JSON.stringify(sputnikProposal(`Accept FACTOR invoice #${nextId}`, FACTOR, "accept_invoice", { id: nextId }, "0", 30), null, 2)
    : null;
  const payJson = nextId
    ? JSON.stringify(sputnikProposal(
        `Pay FACTOR invoice #${nextId}`, MUSDC, "ft_transfer_call",
        { receiver_id: `inv-${nextId}.${FACTOR}`, amount: amountOk ? musdcToBase(amount) : "0", msg: "" }, "1", 200), null, 2)
    : null;

  return (
    <div className="mx-auto max-w-xl">
      <p className="eyebrow">new_invoice</p>
      <h1 className="display mt-3 text-4xl font-semibold tracking-tight">New invoice</h1>
      <div role="tablist" aria-label="Steps" className="mt-6 flex gap-2">
        <button role="tab" aria-selected={tab === "create"} className={tab === "create" ? "btn !min-h-[38px] !px-4" : "btn-ghost !min-h-[38px] !px-4"} onClick={() => setTab("create")}>1 · Create</button>
        <button role="tab" aria-selected={tab === "dao"} className={tab === "dao" ? "btn !min-h-[38px] !px-4" : "btn-ghost !min-h-[38px] !px-4"} onClick={() => setTab("dao")}>2 · DAO approval</button>
      </div>

      {tab === "create" ? (
        <div className="card mt-4 space-y-4 p-5">
          <label className="block text-sm">Payer DAO
            <input className="input mt-1" list="daos" value={dao} onChange={(e) => setDao(e.target.value.trim())} placeholder="your-dao.sputnik-v2.testnet" />
            <datalist id="daos">{DEFAULT_DAO && <option value={DEFAULT_DAO} />}</datalist>
          </label>
          <label className="block text-sm">Amount (mUSDC)
            <input className="input mt-1" inputMode="decimal" value={amount} onChange={(e) => setAmount(e.target.value)} />
            {!amountOk && <span className="text-xs text-red-700">Enter an amount above 0.</span>}
          </label>
          <label className="block text-sm">Due date
            <input className="input mt-1" type="date" value={due} onChange={(e) => setDue(e.target.value)} />
            {dueMs <= Date.now() && <span className="text-xs text-red-700">Pick a future date.</span>}
          </label>
          <label className="block text-sm">Memo
            <input className="input mt-1" value={memo} maxLength={280} onChange={(e) => setMemo(e.target.value)} placeholder="Q4 design work" />
          </label>
          <button className="btn w-full" disabled={busy || (!!account && !valid)} onClick={submit}
            title={account && !valid ? "Fill in all fields" : undefined}>
            {busy ? "Waiting for wallet…" : account ? "Create invoice" : "Connect wallet to create"}
          </button>
        </div>
      ) : (
        <div className="card mt-4 space-y-3 p-5">
          <p className="muted text-sm">Only the DAO can accept. Copy the proposal for invoice #{nextId ?? "…"} and have a council member submit it.</p>
          <div className="flex flex-wrap gap-2">
            <button className="btn-ghost" disabled={!acceptJson} onClick={() => acceptJson && copy(acceptJson, "Accept proposal")}>Copy accept proposal</button>
            <button className="btn-ghost" disabled={!payJson} onClick={() => payJson && copy(payJson, "Pay proposal")}>Copy pay proposal</button>
          </div>
          {acceptJson && <pre className="overflow-x-auto rounded-lg border bg-white p-3 text-xs" style={{ borderColor: "var(--rule)" }}>{acceptJson}</pre>}
        </div>
      )}
    </div>
  );
}
