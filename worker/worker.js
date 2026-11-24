// AI Relay — a private reverse proxy for AI APIs on Cloudflare Workers (your own account).
// Address in your app (custom / OpenAI-compatible connection):  https://<worker-host>/<SECRET>/<service>/<rest>
//   gemini     → https://HOST/SECRET/gemini/v1beta        (format: Gemini)
//   openai     → https://HOST/SECRET/openai/v1            (format: OpenAI)
//   anthropic  → https://HOST/SECRET/anthropic/v1         (format: Anthropic)
//   openrouter → https://HOST/SECRET/openrouter/api/v1    (format: OpenAI)
//   groq       → https://HOST/SECRET/groq/openai/v1       (format: OpenAI)
// SECRET is a Worker secret variable (Settings → Variables and Secrets → type Secret), min 16 chars.
// Without the right secret every request gets a plain 404, so the host looks like an empty site.
// Only the AI hosts below are reachable: this is not an open proxy.
// RELAY_VERSION 1
// Open https://HOST/SECRET/ in a browser to see every service's address (bookmarkable).

const TARGETS = {
  gemini: 'generativelanguage.googleapis.com',
  openai: 'api.openai.com',
  anthropic: 'api.anthropic.com',
  openrouter: 'openrouter.ai',
  groq: 'api.groq.com',
  xai: 'api.x.ai',
  mistral: 'api.mistral.ai',
  deepseek: 'api.deepseek.com',
};

// Headers that must not leak the user's IP or confuse the upstream.
const DROP = ['host', 'cf-connecting-ip', 'cf-ipcountry', 'cf-ray', 'cf-visitor', 'cf-worker',
  'x-forwarded-for', 'x-forwarded-proto', 'x-real-ip', 'true-client-ip', 'cdn-loop', 'origin', 'referer'];

// [name, API format in the app, base path]
const LIST = [
  ['Gemini', 'Gemini', 'gemini/v1beta'], ['ChatGPT (OpenAI)', 'OpenAI', 'openai/v1'],
  ['Claude (Anthropic)', 'Anthropic', 'anthropic/v1'], ['OpenRouter', 'OpenAI', 'openrouter/api/v1'],
  ['Groq', 'OpenAI', 'groq/openai/v1'], ['Grok (xAI)', 'OpenAI', 'xai/v1'],
  ['Mistral', 'OpenAI', 'mistral/v1'], ['DeepSeek', 'OpenAI', 'deepseek/v1'],
];

function listPage(origin, secret) {
  const rows = LIST.map(([n, f, p]) => {
    const u = `${origin}/${secret}/${p}`;
    return `<div class="r"><div><b>${n}</b><small>قالب / format: ${f}</small></div><code>${u}</code><button data-u="${u}">کپی</button></div>`;
  }).join('');
  const html = `<!doctype html><html lang="fa" dir="rtl"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1">
<meta name="robots" content="noindex,nofollow"><meta name="referrer" content="no-referrer"><title>رله‌ی شما</title><style>
body{margin:0;background:#0b0907;color:#f4ecd9;font-family:'Satrap','Vazirmatn','Segoe UI',Tahoma,sans-serif;line-height:1.9;padding:24px 16px}
.w{max-width:760px;margin:auto}h1{color:#f7dfa0;font-size:24px;margin:0 0 6px}p{color:#c2b291;margin:6px 0 16px}
.r{display:grid;grid-template-columns:160px 1fr auto;gap:10px;align-items:center;background:#17120d;border:1px solid rgba(217,179,92,.18);border-radius:12px;padding:10px 12px;margin:8px 0}
.r small{display:block;color:#8a7b62;font-size:12px}code{direction:ltr;text-align:left;font-size:12px;word-break:break-all;color:#f7dfa0}
button{font:inherit;font-weight:700;border:0;border-radius:10px;padding:6px 14px;cursor:pointer;background:linear-gradient(135deg,#f7dfa0,#d9b35c);color:#211608}
.n{border-inline-start:4px solid #e8707f;background:rgba(232,112,127,.08);border-radius:8px;padding:10px 14px;font-size:14px;margin-top:18px}
@media(max-width:600px){.r{grid-template-columns:1fr}}</style></head><body><div class="w">
<h1>آدرس‌های رله‌ی شما</h1><p>در برنامه برای هر سرویس یک «اتصالِ سفارشی» بسازید: قالب و آدرسِ همان ردیف، و کلیدِ API خودِ آن سرویس.</p>
${rows}<div class="n">این صفحه و آدرس‌ها مثلِ رمزند؛ برای کسی نفرستید. این صفحه را نشانک کنید تا هر وقت خواستید دوباره ببینید.</div></div>
<script>document.addEventListener('click',async e=>{const b=e.target.closest('button');if(!b)return;try{await navigator.clipboard.writeText(b.dataset.u)}catch{}b.textContent='✓';setTimeout(()=>b.textContent='کپی',1300)})</script></body></html>`;
  return new Response(html, {
    headers: { 'content-type': 'text/html; charset=utf-8', 'cache-control': 'no-store', 'x-robots-tag': 'noindex', 'referrer-policy': 'no-referrer' },
  });
}

const notFound = () => new Response('Not Found', { status: 404, headers: { 'content-type': 'text/plain' } });

function safeEqual(a, b) {
  if (typeof a !== 'string' || typeof b !== 'string' || a.length !== b.length) return false;
  let r = 0;
  for (let i = 0; i < a.length; i++) r |= a.charCodeAt(i) ^ b.charCodeAt(i);
  return r === 0;
}

export default {
  async fetch(request, env) {
    const secret = env.SECRET;
    if (!secret || secret.length < 16) return new Response('SECRET is not set (min 16 chars)', { status: 500 });

    const url = new URL(request.url);
    const parts = url.pathname.split('/'); // ['', SECRET, service, ...rest]
    if (!safeEqual(parts[1] || '', secret)) return notFound();

    const service = parts[2];
    // Health check: https://HOST/SECRET/ping  or  https://HOST/SECRET/<service>/ping  (readable from a web page)
    if (service === 'ping' || (TARGETS[service] && parts[3] === 'ping' && parts.length === 4)) {
      return new Response(JSON.stringify({ ok: true, relay: 1, colo: request.cf?.colo || '' }), {
        headers: { 'content-type': 'application/json', 'access-control-allow-origin': '*', 'cache-control': 'no-store' },
      });
    }

    // Address list: https://HOST/SECRET/  — open it in a browser any time to copy every service's address.
    if (!service && request.method === 'GET') return listPage(url.origin, secret);

    const host = TARGETS[service];
    if (!host) return notFound();

    const target = `https://${host}/${parts.slice(3).join('/')}${url.search}`;
    const headers = new Headers();
    for (const [k, v] of request.headers) if (!DROP.includes(k.toLowerCase())) headers.set(k, v);

    const init = { method: request.method, headers, redirect: 'manual' };
    if (request.method !== 'GET' && request.method !== 'HEAD') init.body = request.body;

    let resp;
    try {
      resp = await fetch(target, init);
    } catch (e) {
      return new Response(JSON.stringify({ error: 'upstreamFailed', detail: String(e) }), {
        status: 502, headers: { 'content-type': 'application/json' },
      });
    }

    // The body is passed through as a stream (SSE answers arrive piece by piece).
    const out = new Headers(resp.headers);
    out.delete('set-cookie');
    return new Response(resp.body, { status: resp.status, statusText: resp.statusText, headers: out });
  },
};
