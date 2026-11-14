"use client";
import { useEffect, useState } from "react";
import { AGENT_TXS, DEMO_TXS, type DemoTx } from "@/lib/demoTxs";
import { DEFAULT_DAO, EXPLORER, FACTOR, MUSDC, REGISTRY, agentsCount, getNextId, view } from "@/lib/near";

type Tab = "contracts" | "transactions";

const short = (s: string) => (s.length > 22 ? `${s.slice(0, 10)}…${s.slice(-8)}` : s);

function TxList({ title, items }: { title: string; items: DemoTx[] }) {
  return (
    <div className="card px-6 py-2">
      <p className="eyebrow pt-4">{title}</p>
      {items.map((t) => (
        <div key={t.hash} className="kv-row text-sm" style={{ gridTemplateColumns: "1.6fr 1fr" }}>
          <span>{t.label}</span>
          <a className="mono-link underline-draw justify-self-end" href={`${EXPLORER}/txns/${t.hash}`} target="_blank" rel="noreferrer" title={t.hash}>{short(t.hash)} ↗</a>
        </div>
      ))}
    </div>
  );
}

export default function Live() {
  const [tab, setTab] = useState<Tab>("contracts");
  const [invoices, setInvoices] = useState<number | null>(null);
  const [agents, setAgents] = useState<number | null>(null);
  const [vault, setVault] = useState<string>("");
  useEffect(() => {
    getNextId().then((n) => setInvoices(Number(n) - 1)).catch(() => {});
    if (REGISTRY) agentsCount().then(setAgents).catch(() => {});
    view<{ vault_code_hash: string }>(FACTOR, "config").then((c) => setVault(c.vault_code_hash)).catch(() => {});
  }, []);

  const rows: [string, string, string | null, string][] = [
    ["Factor", FACTOR, `${EXPLORER}/address/${FACTOR}`, invoices === null ? "" : `${invoices} invoice${invoices === 1 ? "" : "s"}`],
    ["Agent registry", REGISTRY, `${EXPLORER}/address/${REGISTRY}`, agents === null ? "" : `${agents} registered`],
    ["mUSDC (mock)", MUSDC, `${EXPLORER}/address/${MUSDC}`, ""],
    ["Sputnik DAO", DEFAULT_DAO, `${EXPLORER}/address/${DEFAULT_DAO}`, "2 of 3 council"],
    ["Vault code (global)", vault, null, "shared by new vaults"],
  ];

  return (
    <div>
      <p className="eyebrow">live</p>
      <h1 className="display mt-3 text-4xl font-semibold tracking-tight sm:text-5xl">Live on testnet</h1>
      <div role="tablist" aria-label="Live data" className="mt-8 flex gap-2">
        {(["contracts", "transactions"] as Tab[]).map((t) => (
          <button key={t} role="tab" aria-selected={tab === t} className={tab === t ? "btn" : "btn-ghost"} onClick={() => setTab(t)}>{t}</button>
        ))}
      </div>

      <div className="mt-6">
        {tab === "contracts" ? (
          <div className="card px-6 py-2">
            {rows.map(([name, where, href, note]) => (
              <div key={name} className="kv-row text-sm" style={{ gridTemplateColumns: "1fr 1.6fr 1fr" }}>
                <span>{name}</span>
                {href ? <a className="mono-link underline-draw truncate" href={href} target="_blank" rel="noreferrer">{where} ↗</a> : <span className="mono-link truncate" title={where}>{short(where)}</span>}
                <span className="muted justify-self-end">{note}</span>
              </div>
            ))}
          </div>
        ) : (
          <div className="grid gap-5 lg:grid-cols-2">
            <TxList title="invoice demo" items={DEMO_TXS} />
            <TxList title="agent registry" items={AGENT_TXS} />
          </div>
        )}
      </div>
    </div>
  );
}
