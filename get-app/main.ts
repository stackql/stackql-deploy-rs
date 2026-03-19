const GITHUB_REPO = "stackql/stackql-deploy";

function getAssetName(ua: string): string {
  if (/windows/i.test(ua)) return "stackql-deploy-windows-x86_64.zip";
  if (/darwin|macintosh|mac os/i.test(ua)) return "stackql-deploy-macos-universal.tar.gz";
  return "stackql-deploy-linux-x86_64.tar.gz";
}

Deno.serve((req: Request) => {
  const url = new URL(req.url);

  if (url.pathname !== "/") {
    return Response.redirect("https://stackql-deploy.io", 301);
  }

  const ua = req.headers.get("user-agent") ?? "";
  const asset = getAssetName(ua);
  return Response.redirect(
    `https://github.com/${GITHUB_REPO}/releases/latest/download/${asset}`,
    302
  );
});