import type { InvoiceView } from "@/lib/near";

export default function StatusChip({ inv }: { inv: InvoiceView }) {
  const label = inv.is_overdue ? "OVERDUE" : inv.is_listed ? "LISTED" : inv.status.toUpperCase();
  const tone =
    inv.is_overdue ? "bg-red-500/15 text-red-900"
    : inv.status === "Settled" ? "bg-emerald-500/20 text-emerald-900"
    : inv.status === "Paid" ? "bg-sky-500/15 text-sky-900"
    : inv.is_listed ? "bg-indigo-500/15 text-indigo-900"
    : "bg-black/10 text-stone-800";
  return <span className={`rounded-full px-2 py-0.5 text-xs font-semibold ${tone}`}>{label}</span>;
}
