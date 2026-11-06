// Evaluates an expression in a page reachable over the Chrome DevTools
// Protocol and prints the JSON result, or null.
// node cdp.mjs <port> <url-substring or ""> <expression>
const [, , port, match, expr] = process.argv;
let targets;
try {
  targets = await (await fetch(`http://127.0.0.1:${port}/json`)).json();
} catch {
  console.log("null");
  process.exit(0);
}
const page = targets.find((t) => t.type === "page" && (!match || t.url.includes(match)));
if (!page) {
  console.log("null");
  process.exit(0);
}
const ws = new WebSocket(page.webSocketDebuggerUrl);
await new Promise((r) => ws.addEventListener("open", r));
ws.send(JSON.stringify({ id: 1, method: "Runtime.evaluate", params: { expression: expr, returnByValue: true, awaitPromise: true } }));
ws.addEventListener("message", (m) => {
  const d = JSON.parse(m.data);
  if (d.id === 1) {
    console.log(JSON.stringify(d.result?.result?.value ?? null));
    ws.close();
    process.exit(0);
  }
});
setTimeout(() => {
  console.log("null");
  process.exit(0);
}, 8000);
