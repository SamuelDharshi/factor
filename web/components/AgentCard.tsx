"use client";
// Agent / robot card: staggered entrance, cursor spotlight, animated border, live on-chain stats.
import { motion, useReducedMotion } from "motion/react";
import { useEffect, useRef, useState } from "react";
import { EXPLORER, type AgentView, invoicesHeld, nearBalance, timeAgo, yoctoToNear } from "@/lib/near";

function hue(s: string) {
  let h = 0;
  for (const c of s) h = (h * 31 + c.charCodeAt(0)) % 360;
  return h;
}

export function Orb({ seed, kind, size = 56 }: { seed: string; kind: "Agent" | "Robot"; size?: number }) {
  const h = hue(seed), h2 = (h + 70) % 360;
  const id = `o${h}${size}${kind}`;
  return (
    <svg width={size} height={size} viewBox="0 0 64 64" aria-hidden>
      <defs>
        <radialGradient id={id} cx="35%" cy="30%" r="80%">
          <stop offset="0" stopColor={`hsl(${h} 90% 70%)`} />
          <stop offset="0.55" stopColor={`hsl(${h2} 80% 45%)`} />
          <stop offset="1" stopColor={`hsl(${h2} 70% 12%)`} />
        </radialGradient>
      </defs>
      {kind === "Robot" ? (
        <g>
          <rect x="8" y="12" width="48" height="42" rx="12" fill={`url(#${id})`} />
          <circle cx="23" cy="31" r="5" fill="#04060a" /><circle cx="41" cy="31" r="5" fill="#04060a" />
          <circle cx="23" cy="31" r="2" fill="#00ec97" /><circle cx="41" cy="31" r="2" fill="#00ec97" />
          <rect x="24" y="42" width="16" height="3" rx="1.5" fill="#04060a" opacity=".7" />
          <rect x="30" y="4" width="4" height="9" rx="2" fill={`hsl(${h} 80% 60%)`} />
        </g>
      ) : (
        <g>
          <circle cx="32" cy="32" r="26" fill={`url(#${id})`} />
          <ellipse cx="32" cy="32" rx="26" ry="9" fill="none" stroke="rgba(255,255,255,.35)" strokeWidth="1" transform="rotate(-24 32 32)" />
          <circle cx="50" cy="22" r="3" fill="#00ec97" />
        </g>
      )}
    </svg>
  );
}

const fmtNear = (y: bigint) => yoctoToNear(y, 2);

export default function AgentCard({ a, index = 0, preview = false }: { a: AgentView; index?: number; preview?: boolean }) {
  const ref = useRef<HTMLDivElement>(null);
  const reduce = useReducedMotion();
  const [bal, setBal] = useState<bigint | null>(null);
  const [held, setHeld] = useState<number | null>(null);

  useEffect(() => {
    if (preview) return;
    nearBalance(a.account_id).then(setBal).catch(() => setBal(null));
    invoicesHeld(a.account_id).then(setHeld).catch(() => setHeld(null));
  }, [a.account_id, preview]);

  const onMove = (e: React.PointerEvent) => {
    const el = ref.current;
    if (!el) return;
    const r = el.getBoundingClientRect();
    el.style.setProperty("--mx", `${e.clientX - r.left}px`);
    el.style.setProperty("--my", `${e.clientY - r.top}px`);
  };

  const maxPrice = BigInt(a.strategy.max_price);
  return (
    <motion.article
      ref={ref}
      onPointerMove={onMove}
      className="agent-card group relative flex flex-col gap-4 overflow-hidden rounded-2xl p-5"
      initial={reduce ? false : { opacity: 0, y: 36, scale: 0.97 }}
      whileInView={{ opacity: 1, y: 0, scale: 1 }}
      viewport={{ once: true, margin: "0px 0px -8% 0px" }}
      transition={{ duration: 0.7, delay: (index % 3) * 0.09, ease: [0.22, 1, 0.36, 1] }}
    >
      <div className="relative z-10 flex items-start gap-3">
        <div className="orb-wrap shrink-0"><Orb seed={a.account_id} kind={a.kind} /></div>
        <div className="min-w-0 flex-1">
          <div className="flex items-center gap-2">
            <h3 className="truncate text-lg font-semibold leading-tight">{a.name}</h3>
            <span className={`kind-badge ${a.kind === "Robot" ? "kind-robot" : "kind-agent"}`}>{a.kind === "Robot" ? "ROBOT" : "AGENT"}</span>
          </div>
          <a className="mono-link block truncate" href={`${EXPLORER}/address/${a.account_id}`} target="_blank" rel="noreferrer">{a.account_id}</a>
        </div>
      </div>

      <p className="relative z-10 line-clamp-2 min-h-[2.6rem] text-sm leading-relaxed muted">{a.description || "No description."}</p>

      <ul className="relative z-10 flex flex-wrap gap-1.5">
        {a.capabilities.map((c) => <li key={c} className="chip">{c}</li>)}
      </ul>

      <dl className="relative z-10 grid grid-cols-3 gap-2 text-center">
        <div className="stat"><dt>min score</dt><dd>{a.strategy.min_score}</dd></div>
        <div className="stat"><dt>min APR</dt><dd>{a.strategy.min_apr_pct}%</dd></div>
        <div className="stat"><dt>max (NEAR)</dt><dd>{maxPrice === BigInt(0) ? "—" : fmtNear(maxPrice)}</dd></div>
      </dl>

      <div className="relative z-10 mt-auto flex items-center justify-between border-t pt-3 text-xs muted" style={{ borderColor: "var(--rule)" }}>
        <span title="live NEAR balance">{preview ? "—" : bal === null ? "…" : `${fmtNear(bal)} NEAR`}</span>
        <span title="FACTOR invoice NFTs held">{preview ? "—" : held === null ? "…" : `${held} invoice${held === 1 ? "" : "s"}`}</span>
        <span>{preview ? "just now" : timeAgo(a.registered_ts)}</span>
      </div>

      {a.endpoint && (
        <a className="card-cta relative z-10" href={a.endpoint} target="_blank" rel="noreferrer noopener" aria-label={`Open ${a.name} endpoint`}>
          Open endpoint <span aria-hidden>↗</span>
        </a>
      )}
    </motion.article>
  );
}

export function AgentSkeleton() {
  return (
    <div className="agent-card skeleton flex h-[22rem] flex-col gap-4 rounded-2xl p-5" aria-hidden>
      <div className="flex gap-3"><div className="h-14 w-14 rounded-full bg-black/5" /><div className="flex-1 space-y-2"><div className="h-4 w-2/3 rounded bg-black/5" /><div className="h-3 w-1/2 rounded bg-black/5" /></div></div>
      <div className="h-10 rounded bg-black/5" /><div className="h-6 w-3/4 rounded bg-black/5" /><div className="h-16 rounded bg-black/5" />
    </div>
  );
}
