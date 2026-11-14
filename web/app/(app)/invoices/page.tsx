"use client";
// Marketplace: every card is a real listing read from the factor contract.
import Link from "next/link";
import { useCallback, useEffect, useState } from "react";
import ScoreBadge from "@/components/ScoreBadge";
import {
  FACTOR, daysToDue, discountPct, faceInNear, fmtMusdc, ftBalance, getListings, getStats, impliedApr,
  type InvoiceView, type PayerStatsView, yoctoToNear,
} from "@/lib/near";
import { useWallet } from "@/lib/wallet";

interface Row { inv: InvoiceView; stats: PayerStatsView; treasury: bigint }

export default function Marketplace() {
  const { account, connect, call } = useWallet();
  const [rows, setRows] = useState<Row[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [minApr, setMinApr] = useState("0");
  const [minScore, setMinScore] = useState("0");
  const [dueWithin, setDueWithin] = useState("365");

  const load = useCallback(async () => {
    try {
      const listings = await getListings();
      const out = await Promise.all(
        listings.map(async (inv) => {
          const [stats, treasury] = await Promise.all([getStats(inv.payer_dao), ftBalance(inv.payer_dao)]);
          return { inv, stats, treasury };
        }),
      );
      setRows(out);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    }
  }, []);
  useEffect(() => { load(); }, [load]);

  const visible = (rows ?? []).filter(({ inv, stats }) =>
    (impliedApr(inv) ?? 0) >= Number(minApr || 0) &&
    stats.payer_score >= Number(minScore || 0) &&
    daysToDue(inv) <= Number(dueWithin || 36500));

  async function buy(inv: InvoiceView) {
    if (!account) return connect();
    await call(FACTOR, "buy", { id: inv.id }, inv.price!, 50, `Buy invoice #${inv.id}`);
  }

  return (
    <div>
      <p className="eyebrow">invoices</p>
      <h1 className="display mt-3 text-4xl font-semibold tracking-tight sm:text-5xl">For sale</h1>

      <div className="card mt-4 grid grid-cols-1 gap-3 p-4 sm:grid-cols-3">
        <label className="text-sm">Min implied APR (%)
          <input className="input mt-1" inputMode="numeric" value={minApr} onChange={(e) => setMinApr(e.target.value)} /></label>
        <label className="text-sm">Min payer score
          <input className="input mt-1" inputMode="numeric" value={minScore} onChange={(e) => setMinScore(e.target.value)} /></label>
        <label className="text-sm">Due within (days)
          <input className="input mt-1" inputMode="numeric" value={dueWithin} onChange={(e) => setDueWithin(e.target.value)} /></label>
      </div>

      {error && <p className="mt-4 text-sm text-red-700">Could not load listings: {error}</p>}
      {!rows && !error && <p className="muted mt-6 text-sm">Loading…</p>}
      {rows && rows.length === 0 && (
        <p className="muted mt-6 text-sm">Nothing listed yet. <Link className="underline" href="/new">Create an invoice</Link>.</p>
      )}
      {rows && rows.length > 0 && visible.length === 0 && <p className="muted mt-6 text-sm">No listing matches these filters.</p>}

      <div className="mt-4 grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
        {visible.map(({ inv, stats, treasury }) => {
          const apr = impliedApr(inv)!;
          const disc = discountPct(inv)!;
          const own = account === inv.owner;
          return (
            <div key={inv.id} className="card flex flex-col gap-3 p-4">
              <div className="flex items-center justify-between">
                <Link href={`/invoice/${inv.id}`} className="font-semibold hover:underline">Invoice #{inv.id}</Link>
                <ScoreBadge score={stats.payer_score} />
              </div>
              <div className="muted text-xs break-all">{inv.payer_dao}</div>
              <dl className="grid grid-cols-2 gap-y-1 text-sm">
                <dt className="muted">Face</dt><dd>{fmtMusdc(inv.amount)} mUSDC <span className="muted">(≈{faceInNear(inv).toFixed(2)} NEAR)</span></dd>
                <dt className="muted">Price</dt><dd>{yoctoToNear(inv.price!)} NEAR</dd>
                <dt className="muted">Discount</dt><dd>{disc.toFixed(1)}%</dd>
                <dt className="muted">Implied APR</dt><dd>{apr.toFixed(0)}%</dd>
                <dt className="muted">Due in</dt><dd>{Math.max(daysToDue(inv), 0).toFixed(0)} days{inv.is_overdue ? " (overdue)" : ""}</dd>
                <dt className="muted">DAO treasury</dt><dd>{fmtMusdc(treasury)} mUSDC</dd>
                <dt className="muted">Track record</dt><dd>{stats.paid_on_time}/{stats.paid_on_time + stats.paid_late} on time</dd>
              </dl>
              <button
                className="btn mt-auto"
                disabled={own}
                title={own ? "You own this invoice" : undefined}
                onClick={() => buy(inv)}
              >
                {own ? "Your listing" : account ? `Buy for ${yoctoToNear(inv.price!)} NEAR` : "Connect wallet to buy"}
              </button>
            </div>
          );
        })}
      </div>
    </div>
  );
}
