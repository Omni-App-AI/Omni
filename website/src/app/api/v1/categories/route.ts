import { NextResponse } from "next/server";
import { CATEGORIES } from "@/lib/constants";
import { createServiceClient } from "@/lib/supabase/server";

export async function GET() {
  const supabase = createServiceClient();

  // Get extension count per category
  const { data: extensions } = await supabase
    .from("extensions")
    .select("categories")
    .eq("published", true);

  const categoryCounts: Record<string, number> = {};
  (extensions as { categories: string[] }[] | null)?.forEach((ext) => {
    ext.categories.forEach((cat) => {
      categoryCounts[cat] = (categoryCounts[cat] || 0) + 1;
    });
  });

  const categories = CATEGORIES.map((cat) => ({
    id: cat.id,
    name: cat.name,
    icon: cat.icon,
    count: categoryCounts[cat.id] || 0,
  }));

  return NextResponse.json({ categories });
}
