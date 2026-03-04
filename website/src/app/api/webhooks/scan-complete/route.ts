import { NextResponse, type NextRequest } from "next/server";

export async function POST(request: NextRequest) {
  // Verify webhook secret
  const secret = request.headers.get("x-webhook-secret");
  if (secret !== process.env.WEBHOOK_SECRET) {
    return NextResponse.json({ error: "Unauthorized" }, { status: 401 });
  }

  const payload = await request.json();
  const { version_id, extension_id, verdict, overall_score } = payload;

  // This webhook is called by the scan-extension edge function
  // when a scan completes. It can be used to:
  // 1. Send email notifications to publishers
  // 2. Trigger cache invalidation
  // 3. Update real-time dashboards via WebSocket

  console.log(
    `Scan complete: ${extension_id} v${payload.version} — ${verdict} (${overall_score}/100)`,
  );

  return NextResponse.json({ received: true });
}
