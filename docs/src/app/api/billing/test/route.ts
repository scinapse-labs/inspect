import { NextResponse } from "next/server";

export async function GET() {
  const key = process.env.STRIPE_SECRET_KEY;
  if (!key) {
    return NextResponse.json({ ok: false, error: "No STRIPE_SECRET_KEY" });
  }

  // Test 1: Raw fetch to Stripe API (bypass SDK)
  try {
    const res = await fetch("https://api.stripe.com/v1/balance", {
      headers: { Authorization: `Bearer ${key}` },
    });
    const data = await res.json();
    if (res.ok) {
      return NextResponse.json({ ok: true, method: "raw_fetch", currency: data.available?.[0]?.currency });
    }
    return NextResponse.json({ ok: false, method: "raw_fetch", status: res.status, error: data.error?.message });
  } catch (e: any) {
    return NextResponse.json({ ok: false, method: "raw_fetch", error: e.message });
  }
}
