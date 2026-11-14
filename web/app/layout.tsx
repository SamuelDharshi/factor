import type { Metadata, Viewport } from "next";
import { Bricolage_Grotesque, Geist_Mono, Instrument_Serif, Space_Grotesk } from "next/font/google";
import Script from "next/script";
import "./globals.css";
import { WalletProvider } from "@/lib/wallet";
import Shell from "@/components/Shell";
import Footer from "@/components/Footer";
import { INTRO_BOOT } from "@/components/Intro";

// hero: Instrument Serif · titles: Bricolage Grotesque · subtitles/body: Space Grotesk · numbers: Geist Mono
const hero = Instrument_Serif({ subsets: ["latin"], weight: "400", style: ["normal", "italic"], variable: "--font-hero", display: "swap" });
const title = Bricolage_Grotesque({ subsets: ["latin"], variable: "--font-title", display: "swap" });
const sub = Space_Grotesk({ subsets: ["latin"], variable: "--font-sub", display: "swap" });
const mono = Geist_Mono({ subsets: ["latin"], variable: "--font-mono", display: "swap" });

export const metadata: Metadata = {
  title: "FACTOR on NEAR · Invoices that own their money",
  description:
    "DAO-approved invoices as NFTs that own a keyless NEAR vault account, and an on-chain registry where AI agents and robots register to buy them. Testnet demo.",
};

export const viewport: Viewport = { themeColor: "#ebebeb" };

export default function RootLayout({ children }: { children: React.ReactNode }) {
  return (
    <html lang="en" className={`${hero.variable} ${title.variable} ${sub.variable} ${mono.variable}`} suppressHydrationWarning>
      <head suppressHydrationWarning />
      <body suppressHydrationWarning>
        {/* runs before hydration; wallet extensions may also inject scripts, so no hydration diff is raised */}
        <Script id="intro-boot" strategy="beforeInteractive" dangerouslySetInnerHTML={{ __html: INTRO_BOOT }} />
        <WalletProvider>
          <Shell />
          {children}
          <Footer />
        </WalletProvider>
      </body>
    </html>
  );
}
