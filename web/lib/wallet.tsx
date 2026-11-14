"use client";
// Wallet Selector (MyNearWallet + Meteor, testnet) + toast with explorer link after every tx.
import { createContext, useCallback, useContext, useEffect, useRef, useState, type ReactNode } from "react";
import "@near-wallet-selector/modal-ui/styles.css";
import type { WalletSelector } from "@near-wallet-selector/core";
import { actionCreators } from "@near-js/transactions";
import { EXPLORER, FACTOR } from "./near";

export interface Toast { kind: "ok" | "err" | "info"; text: string; hash?: string }
interface Ctx {
  account: string | null;
  ready: boolean;
  connect: () => void;
  disconnect: () => Promise<void>;
  /** Sign + send one FunctionCall. Resolves to the tx hash when the wallet returns it. */
  call: (receiver: string, method: string, args: object, depositYocto: string, tgas: number, label: string) => Promise<string | null>;
  toast: Toast | null;
  notify: (t: Toast | null) => void;
}
const WalletCtx = createContext<Ctx | null>(null);
export const useWallet = () => {
  const c = useContext(WalletCtx);
  if (!c) throw new Error("WalletProvider missing");
  return c;
};

export function WalletProvider({ children }: { children: ReactNode }) {
  const selector = useRef<WalletSelector | null>(null);
  const modal = useRef<{ show: () => void } | null>(null);
  const [account, setAccount] = useState<string | null>(null);
  const [ready, setReady] = useState(false);
  const [toast, setToast] = useState<Toast | null>(null);

  useEffect(() => {
    let off: (() => void) | undefined;
    (async () => {
      const [{ setupWalletSelector }, { setupModal }, { setupMyNearWallet }, { setupMeteorWallet }] = await Promise.all([
        import("@near-wallet-selector/core"),
        import("@near-wallet-selector/modal-ui"),
        import("@near-wallet-selector/my-near-wallet"),
        import("@near-wallet-selector/meteor-wallet"),
      ]);
      const s = await setupWalletSelector({ network: "testnet", modules: [setupMyNearWallet(), setupMeteorWallet()] });
      selector.current = s;
      modal.current = setupModal(s, { contractId: FACTOR || "factor.testnet" });
      const sync = () => setAccount(s.store.getState().accounts.find((a) => a.active)?.accountId ?? null);
      sync();
      const sub = s.store.observable.subscribe(sync);
      off = () => sub.unsubscribe();
      setReady(true);
      // browser wallets redirect back with ?transactionHashes=...
      const q = new URLSearchParams(window.location.search);
      const h = q.get("transactionHashes");
      if (h) {
        setToast({ kind: "ok", text: "Transaction sent", hash: h.split(",").pop() });
        window.history.replaceState({}, "", window.location.pathname);
      }
    })().catch((e) => setToast({ kind: "err", text: `Wallet init failed: ${e}` }));
    return () => off?.();
  }, []);

  const connect = useCallback(() => modal.current?.show(), []);
  const disconnect = useCallback(async () => {
    const w = await selector.current?.wallet();
    await w?.signOut();
    setAccount(null);
  }, []);

  const call: Ctx["call"] = useCallback(async (receiver, method, args, deposit, tgas, label) => {
    if (!selector.current) return null;
    try {
      const wallet = await selector.current.wallet();
      const out = await wallet.signAndSendTransaction({
        receiverId: receiver,
        actions: [actionCreators.functionCall(method, args, BigInt(tgas) * BigInt("1000000000000"), BigInt(deposit))],
      });
      const hash = (out as { transaction?: { hash?: string } } | void)?.transaction?.hash ?? undefined;
      setToast({ kind: "ok", text: `${label} sent`, hash });
      return hash ?? null;
    } catch (e) {
      setToast({ kind: "err", text: `${label} failed: ${e instanceof Error ? e.message : String(e)}` });
      return null;
    }
  }, []);

  return (
    <WalletCtx.Provider value={{ account, ready, connect, disconnect, call, toast, notify: setToast }}>
      {children}
      {toast && (
        <div
          role="status"
          className={`fixed bottom-4 right-4 max-w-sm rounded-lg border px-4 py-3 text-sm shadow-lg ${
            toast.kind === "err" ? "border-red-700/40 bg-white text-red-900" : "border-emerald-700/40 bg-white text-emerald-900"
          }`}
        >
          <div className="flex items-start gap-3">
            <div className="flex-1">
              <div>{toast.text}</div>
              {toast.hash && (
                <a className="underline" href={`${EXPLORER}/txns/${toast.hash}`} target="_blank" rel="noreferrer">
                  View on explorer ↗
                </a>
              )}
            </div>
            <button aria-label="dismiss" onClick={() => setToast(null)} className="opacity-60 hover:opacity-100">✕</button>
          </div>
        </div>
      )}
    </WalletCtx.Provider>
  );
}
