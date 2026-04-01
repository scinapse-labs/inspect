import { auth } from "@clerk/nextjs/server";
import { NextResponse } from "next/server";
import { getSupabase } from "@/lib/supabase";

const SIGNUP_BONUS_CENTS = 5_00; // $5 free credits

export async function GET() {
  const { userId } = await auth();
  if (!userId) {
    return NextResponse.json({ error: "Unauthorized" }, { status: 401 });
  }

  const supabase = getSupabase();
  const { data } = await supabase
    .from("credits")
    .select("balance_cents")
    .eq("user_id", userId)
    .single();

  // New user: grant $5 signup bonus
  if (!data) {
    await supabase.from("credits").insert({
      user_id: userId,
      balance_cents: SIGNUP_BONUS_CENTS,
    });

    await supabase.from("credit_transactions").insert({
      user_id: userId,
      amount_cents: SIGNUP_BONUS_CENTS,
      type: "signup_bonus",
    });

    return NextResponse.json({ balance_cents: SIGNUP_BONUS_CENTS });
  }

  // Self-heal: if balance is 0 but transactions exist, recalculate
  if (data.balance_cents === 0) {
    const { data: txns } = await supabase
      .from("credit_transactions")
      .select("amount_cents")
      .eq("user_id", userId);
    const sum = (txns || []).reduce((s: number, t: { amount_cents: number }) => s + t.amount_cents, 0);
    if (sum > 0) {
      await supabase
        .from("credits")
        .update({ balance_cents: sum })
        .eq("user_id", userId);
      return NextResponse.json({ balance_cents: sum });
    }
  }

  return NextResponse.json({
    balance_cents: data.balance_cents || 0,
  });
}
