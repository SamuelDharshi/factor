"use client";
import { use, useCallback, useEffect, useState } from "react";
import ScoreBadge from "@/components/ScoreBadge";
import StatusChip from "@/components/StatusChip";
import {
  EXPLORER, FACTOR, MUSDC, accessKeyCount, daysToDue, discountPct, faceInNear, fmtMusdc, ftBalance, getInvoice,
  getStats, impliedApr, nearToYocto, nsToDate, sputnikProposal, yoctoToNear, type InvoiceView, type PayerStatsView,
} from "@/lib/near";
import { useWallet } from "@/lib/wallet";

const ONE_YOCTO = "1";

export default function InvoicePage({ params }: { params: Promise<{ id: string }> }) {
  const { id } = use(params);
  const { account, connect, call, notify } = useWallet();
  const [inv, setInv] = useState<InvoiceView | null | undefined>(undefined);
  const [stats, setStats] = useState<PayerStatsView | null>(null);
  const [treasury, setTreasury] = useState<bigint | null>(null);
  const [keys, setKeys] = useState<number | null | "n/a">(null);
  const [vaultBal, setVaultBal] = useState<bigint | null>(null);
  const [price, setPrice] = useState("9.5");
  const [err, setErr] = useState<string | null>(null);
  const [tab, setTab] = useState<"overview" | "trade" | "dao">("overview");

  const load = useCallback(async () => {
    try {
      const i = await getInvoice(id);
      setInv(i);
      if (!i) return;
      getStats(i.payer_dao).then(setStats).catch(() => setStats(null));
      ftBalance(i.payer_dao).then(setTreasury).catch(() => setTreasury(null));
      ftBalance(i.vault).then(setVaultBal).catch(() => setVaultBal(null));
      accessKeyCount(i.vault).then(setKeys).catch(() => setKeys("n/a")); // real RPC view_access_key_list
    } catch (e) {
      setErr(e instanceof Error ? e.message : String(e));
    }
  }, [id]);
  useEffect(() => { load(); }, [load]);

  if (err) return <p className="text-sm text-red-700">Could not load invoice: {err}</p>;
  if (inv === undefined) return <p className="muted text-sm">Loading…</p>;
  if (inv === null) return <p className="text-sm">Invoice #{id} does not exist.</p>;

  const isOwner = account === inv.owner;
  const isDao = account === inv.payer_dao;
  const priceYocto = (() => { try { return nearToYocto(price); } catch { return null; } })();
  const disc = discountPct(inv);
  const apr = impliedApr(inv);

  // action availability + the reason shown in the tooltip when disabled
  const why = {
    list: !account ? "Connect your wallet" : !isOwner ? "Only the NFT owner can list" : inv.status !== "Accepted" ? "Only ACCEPTED invoices can be listed" : inv.pending ? "A transfer is in flight" : inv.is_listed ? "Already listed" : !priceYocto || priceYocto === "0" ? "Enter a price" : "",
    delist: !account ? "Connect your wallet" : !isOwner ? "Only the NFT owner can delist" : !inv.is_listed ? "Not listed" : "",
    withdraw: !account ? "Connect your wallet" : !isOwner ? "Only the current NFT owner can withdraw" : inv.status !== "Paid" ? (inv.status === "Settled" ? "Already withdrawn" : "Not paid yet") : inv.pending ? "A withdraw is in flight" : "",
  };

  async function act(method: string, args: object, deposit: string, tgas: number, label: string) {
    if (!account) return connect();
    if (await call(FACTOR, method, args, deposit, tgas, label)) setTimeout(load, 4000);
  }

  async function copyProposal(kind: "accept" | "pay") {
    const body = kind === "accept"
      ? sputnikProposal(`Accept FACTOR invoice #${inv!.id}`, FACTOR, "accept_invoice", { id: inv!.id }, "0", 30)
      : sputnikProposal(`Pay FACTOR invoice #${inv!.id}`, MUSDC, "ft_transfer_call", { receiver_id: inv!.vault, amount: inv!.amount, msg: "" }, "1", 200);
    try {
      await navigator.clipboard.writeText(JSON.stringify(body, null, 2));
      notify({ kind: "ok", text: "Proposal JSON copied. Use it as the args of add_proposal on the DAO." });
    } catch {
      notify({ kind: "err", text: "Clipboard unavailable in this browser context." });
    }
  }

  const steps: { label: string; done: boolean; at: string | null }[] = [
    { label: "Created", done: true, at: inv.created_ts },
    { label: "Accepted by DAO", done: inv.status !== "Draft", at: inv.accepted_ts },
    { label: "Paid by DAO", done: inv.status === "Paid" || inv.status === "Settled", at: inv.paid_ts },
    { label: "Settled (withdrawn)", done: inv.status === "Settled", at: inv.settled_ts },
  ];

  return (
    <div className="grid gap-4 lg:grid-cols-3">
      <section className="card p-4 lg:col-span-2">
        <div className="flex items-center gap-3">
          <h1 className="text-xl font-bold">Invoice #{inv.id}</h1>
          <StatusChip inv={inv} />
          {inv.pending && <span className="text-xs text-amber-800">withdraw in flight</span>}
        </div>
        <div role="tablist" aria-label="Invoice sections" className="mt-4 flex gap-2">
          {([["overview", "Overview"], ["trade", "Trade"], ["dao", "DAO"]] as const).map(([k, label]) => (
            <button key={k} role="tab" aria-selected={tab === k} className={tab === k ? "btn !min-h-[38px] !px-4" : "btn-ghost !min-h-[38px] !px-4"} onClick={() => setTab(k)}>{label}</button>
          ))}
        </div>
        {tab === "overview" && (<>
        <dl className="mt-3 grid grid-cols-[8rem_1fr] gap-y-1 text-sm">
          <dt className="muted">Face value</dt><dd>{fmtMusdc(inv.amount)} mUSDC <span className="muted">(≈{faceInNear(inv).toFixed(2)} NEAR)</span></dd>
          <dt className="muted">Payer DAO</dt><dd className="break-all">{inv.payer_dao}</dd>
          <dt className="muted">Owner</dt><dd className="break-all">{inv.owner}{isOwner ? " (you)" : ""}</dd>
          <dt className="muted">Due</dt><dd>{nsToDate(inv.due_ts).toDateString()} ({Math.round(daysToDue(inv))} days)</dd>
          <dt className="muted">Memo</dt><dd>{inv.memo || "—"}</dd>
          {inv.is_listed && (<><dt className="muted">Listed</dt><dd>{yoctoToNear(inv.price!)} NEAR · {disc?.toFixed(0)}% off · {apr?.toFixed(0)}% APR</dd></>)}
        </dl>

        <h2 className="mt-6 font-semibold">Timeline</h2>
        <ol className="mt-2 space-y-1 text-sm">
          {steps.map((s) => (
            <li key={s.label} className={s.done ? "" : "muted"}>
              {s.done ? "●" : "○"} {s.label}
              {s.done && s.at && <span className="muted"> — {nsToDate(s.at).toLocaleString()}</span>}
            </li>
          ))}
        </ol>
        <p className="mt-3 text-xs">
          <a className="underline-draw" href={`${EXPLORER}/address/${inv.vault}`} target="_blank" rel="noreferrer">Vault on explorer ↗</a>
          <span className="muted"> · </span>
          <a className="underline-draw" href={`${EXPLORER}/address/${FACTOR}`} target="_blank" rel="noreferrer">Factor on explorer ↗</a>
        </p>

        </>)}
        {tab === "trade" && (<>
        <h2 className="mt-6 font-semibold">Trade</h2>
        <div className="mt-2 flex flex-wrap items-end gap-3">
          <label className="text-xs">Price (NEAR)
            <input className="input mt-1 w-28" inputMode="decimal" value={price} onChange={(e) => setPrice(e.target.value)} /></label>
          <button className="btn" disabled={!!why.list} title={why.list || undefined}
            onClick={() => act("list", { id: inv.id, price: priceYocto }, ONE_YOCTO, 30, `List #${inv.id}`)}>List</button>
          <button className="btn-ghost" disabled={!!why.delist} title={why.delist || undefined}
            onClick={() => act("delist", { id: inv.id }, ONE_YOCTO, 30, `Delist #${inv.id}`)}>Delist</button>
          <button className="btn" disabled={!!why.withdraw} title={why.withdraw || undefined}
            onClick={() => act("withdraw", { id: inv.id }, ONE_YOCTO, 150, `Withdraw #${inv.id}`)}>Withdraw {fmtMusdc(inv.amount)} mUSDC</button>
        </div>
        {!account && <p className="muted mt-2 text-xs">Connect a wallet to trade.</p>}

        </>)}
        {tab === "dao" && (<>
        <h2 className="mt-6 font-semibold">For the DAO</h2>
        <p className="muted text-sm">Copy a Sputnik proposal for your DAO to submit.</p>
        <div className="mt-2 flex flex-wrap gap-2">
          <button className="btn-ghost" disabled={inv.status !== "Draft"} title={inv.status !== "Draft" ? "Already accepted" : undefined} onClick={() => copyProposal("accept")}>Copy accept proposal</button>
          <button className="btn-ghost" disabled={inv.status !== "Accepted"} title={inv.status !== "Accepted" ? "Invoice must be ACCEPTED to be paid" : undefined} onClick={() => copyProposal("pay")}>Copy pay proposal</button>
        </div>
        </>)}
      </section>

      <aside className="space-y-4">
        <div className="card p-4">
          <h2 className="font-semibold">Vault</h2>
          <div className="mt-1 break-all text-sm">{inv.vault}</div>
          <div className="mt-2 text-sm">
            {keys === null ? <span className="muted">checking keys…</span>
              : keys === "n/a" ? <span className="muted">access keys: could not check</span>
              : keys === 0 ? <span className="font-semibold text-emerald-700">0 access keys ✓</span>
              : <span className="font-semibold text-red-700">{keys} access keys ✗</span>}
          </div>
          <div className="muted mt-1 text-sm">Holds {vaultBal === null ? "…" : fmtMusdc(vaultBal)} mUSDC</div>
          <p className="muted mt-2 text-xs">Checked live on-chain.</p>
        </div>
        <div className="card p-4">
          <h2 className="font-semibold">Payer</h2>
          <div className="mt-1 break-all text-sm">{inv.payer_dao}</div>
          {stats ? (
            <dl className="mt-2 grid grid-cols-2 gap-y-1 text-sm">
              <dt className="muted">Score</dt><dd><ScoreBadge score={stats.payer_score} /></dd>
              <dt className="muted">On time</dt><dd>{stats.paid_on_time}/{stats.paid_on_time + stats.paid_late}</dd>
              <dt className="muted">Accepted</dt><dd>{stats.accepted}</dd>
              <dt className="muted">Volume paid</dt><dd>{fmtMusdc(stats.volume_paid)} mUSDC</dd>
              <dt className="muted">On FACTOR</dt><dd>{stats.age_days} days</dd>
              <dt className="muted">Treasury</dt><dd>{treasury === null ? "…" : `${fmtMusdc(treasury)} mUSDC`}</dd>
            </dl>
          ) : <p className="muted mt-2 text-sm">Loading…</p>}
        </div>
      </aside>
    </div>
  );
}
