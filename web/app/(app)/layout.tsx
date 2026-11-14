import { DEPLOYED } from "@/lib/near";
import { ClearIntro } from "@/components/Intro";

export default function AppLayout({ children }: { children: React.ReactNode }) {
  return (
    <div className="page">
      <ClearIntro />
      {DEPLOYED ? (
        children
      ) : (
        <div className="card p-6">
          <h2 className="text-lg font-semibold">Contracts not deployed yet</h2>
          <p className="muted mt-2 text-sm">
            Run <code>make deploy dao</code>, then <code>bash scripts/sync-web.sh</code> to fill <code>web/lib/deployments.json</code>.
          </p>
        </div>
      )}
    </div>
  );
}
