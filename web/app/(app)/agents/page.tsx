"use client";
// Agent marketplace: every card is a real registration read from the registry contract.
import Link from "next/link";
import { LayoutGroup, motion } from "motion/react";
import { useEffect, useMemo, useState } from "react";
import AgentCard, { AgentSkeleton } from "@/components/AgentCard";
import { REGISTRY, type AgentKind, type AgentView, listAgents } from "@/lib/near";

type Filter = "All" | AgentKind;
const FILTERS: { v: Filter; label: string }[] = [
  { v: "All", label: "all" },
  { v: "Agent", label: "agents" },
  { v: "Robot", label: "robots" },
];
type Sort = "newest" | "score" | "price";

export default function AgentsPage() {
  const [agents, setAgents] = useState<AgentView[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [filter, setFilter] = useState<Filter>("All");
  const [q, setQ] = useState("");
  const [sort, setSort] = useState<Sort>("newest");

  useEffect(() => {
    if (!REGISTRY) { setAgents([]); return; }
    listAgents().then(setAgents).catch((e) => setError(e instanceof Error ? e.message : String(e)));
  }, []);

  const shown = useMemo(() => {
    const term = q.trim().toLowerCase();
    const list = (agents ?? []).filter((a) =>
      (filter === "All" || a.kind === filter) &&
      (!term || [a.name, a.account_id, a.description, ...a.capabilities].some((s) => s.toLowerCase().includes(term))));
    if (sort === "newest") return [...list].reverse();
    if (sort === "score") return [...list].sort((a, b) => b.strategy.min_score - a.strategy.min_score);
    return [...list].sort((a, b) => Number(BigInt(b.strategy.max_price) - BigInt(a.strategy.max_price)));
  }, [agents, filter, q, sort]);

  const counts = { agents: (agents ?? []).filter((a) => a.kind === "Agent").length, robots: (agents ?? []).filter((a) => a.kind === "Robot").length };

  return (
    <div>
      <p className="eyebrow">agent_market</p>
      <h1 className="display mt-3 text-4xl font-semibold tracking-tight sm:text-5xl">Agents</h1>
      <p className="mt-2 muted">{agents ? `${counts.agents} agents · ${counts.robots} robots` : "Loading…"}</p>

      <div className="mt-8 flex flex-col gap-3 md:flex-row md:items-center">
        <LayoutGroup>
          <div role="tablist" aria-label="Filter by kind" className="flex rounded-xl border p-1" style={{ borderColor: "var(--rule)", background: "#fff" }}>
            {FILTERS.map((f) => (
              <button key={f.v} role="tab" aria-selected={filter === f.v} onClick={() => setFilter(f.v)} className="relative min-h-[40px] rounded-lg px-4 font-mono text-sm">
                {filter === f.v && <motion.span layoutId="pill" className="absolute inset-0 rounded-lg" style={{ background: "var(--ink)" }} transition={{ type: "spring", stiffness: 500, damping: 36 }} />}
                <span className="relative" style={{ color: filter === f.v ? "#fff" : "var(--muted)" }}>{f.label}</span>
              </button>
            ))}
          </div>
        </LayoutGroup>
        <input className="input md:max-w-xs" placeholder="search name, account, capability" value={q} onChange={(e) => setQ(e.target.value)} aria-label="Search agents" />
        <select className="input md:max-w-[11rem]" value={sort} onChange={(e) => setSort(e.target.value as Sort)} aria-label="Sort">
          <option value="newest">newest</option>
          <option value="score">highest min score</option>
          <option value="price">highest max price</option>
        </select>
        <Link href="/agents/register" className="btn md:ml-auto">Register an agent</Link>
      </div>

      {error && <p className="mt-6 text-sm" style={{ color: "var(--bad)" }}>Could not load the registry: {error}</p>}

      <div className="mt-8 grid gap-5 sm:grid-cols-2 lg:grid-cols-3">
        {!agents && !error && [0, 1, 2].map((i) => <AgentSkeleton key={i} />)}
        {agents && shown.map((a, i) => <AgentCard key={a.account_id} a={a} index={i} />)}
      </div>

      {agents && shown.length === 0 && (
        <div className="card mt-8 p-8 text-center">
          <p className="text-lg font-semibold">{agents.length === 0 ? "No agents yet." : "No matches."}</p>
          <p className="mt-2 muted">{agents.length === 0 ? "Be the first to register." : "Try another search."}</p>
          <Link href="/agents/register" className="btn mt-5">Register an agent</Link>
        </div>
      )}
    </div>
  );
}
