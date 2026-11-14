// Real testnet transactions from the `make demo` run (docs/demo-output.txt) and the registry seed script.
export interface DemoTx { label: string; hash: string }

export const DEMO_TXS: DemoTx[] = [
  { label: "Alice creates invoice #1 (keyless vault)", hash: "97QimPKqbwYAe2Eae8QqshPjqGQHHQNjQupRHHWTMz9v" },
  { label: "DAO votes 2/3, accepts the invoice", hash: "8kGfpbUPZZdzgoyVeSXG19WQeBjmEKTsWEmAZm5Ucbe" },
  { label: "Alice lists it at 9.5 NEAR", hash: "7JRV8AwRvAcR1Ms9ApkYofVBZTmaBXixmccz3hty1rmq" },
  { label: "Carol buys it", hash: "6Fe9Ar5v67oxXMNL5NBRKeUDqpK9aR8fvvvqSfUhV1vX" },
  { label: "DAO pays 500 mUSDC to the vault", hash: "B3FcFjCv7jUHqjiwtwLeMKBdpDkHfuHB4ZbmAEF3Q1Rs" },
  { label: "Alice withdraws: rejected (not owner)", hash: "mTh2Pf5oja7aVNC62epE52HxTbfpUDeLZegBWTQfTFG" },
  { label: "Carol withdraws 500 mUSDC", hash: "9W4B8iQvp74TAgBL8PHDseu3jTMTyPZ294U7iDyycKBk" },
];

export const AGENT_TXS: DemoTx[] = [
  { label: "Carol Capital registers (agent)", hash: "AEtCjxJjVkJJahRKWyk1uJqtwFWBLSKcSayJYye11Wzn" },
  { label: "Scout registers (agent)", hash: "Buj4y4SCvRPbW7T75XaWup5chz8RnvZ6BzECZ8JWiVai" },
  { label: "Rover-1 registers (robot)", hash: "2fctxog18wM2mMhHuLNnGq6jLGf759c2hCLuxdvU2DcC" },
];
