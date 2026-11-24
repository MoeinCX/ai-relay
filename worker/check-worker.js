// AI Relay — connection check.
// A tiny public worker on workers.dev. The setup page and the tool call it BEFORE building a relay,
// to see whether workers.dev is reachable from the user's internet without a VPN.
// Opened in a browser it shows a friendly result page; called from code it answers JSON.
// It stores nothing and proxies nothing. It only reports the country of the request
// (from Cloudflare, not stored) so the page can warn when a VPN seems to be on.

const GUIDE = 'https://satraptradingacademy.com/satrapion/ai-relay/';

const CITY = {
  FRA: 'فرانکفورت', AMS: 'آمستردام', CDG: 'پاریس', LHR: 'لندن', IST: 'استانبول', VIE: 'وین',
  ARN: 'استکهلم', WAW: 'ورشو', MXP: 'میلان', MAD: 'مادرید', ZRH: 'زوریخ', DXB: 'دبی', FJR: 'فجیره',
  DOH: 'دوحه', BAH: 'بحرین', KWI: 'کویت', MCT: 'مسقط', EVN: 'ایروان', TBS: 'تفلیس', GYD: 'باکو',
};

function page(colo, country) {
  const where = CITY[colo] || colo || 'نامعلوم';
  const vpn = country && country !== 'IR';
  return `<!doctype html><html lang="fa" dir="rtl"><head><meta charset="utf-8">
<meta name="viewport" content="width=device-width,initial-scale=1"><meta name="robots" content="noindex">
<title>آزمایشِ اتصال</title><style>
@font-face{font-family:Vazirmatn;font-weight:400 800;font-display:swap;src:url(https://cdn.jsdelivr.net/npm/vazirmatn@33.0.3/fonts/webfonts/Vazirmatn[wght].woff2) format('woff2')}
*{box-sizing:border-box;margin:0}
body{min-height:100vh;display:grid;place-items:center;padding:24px 16px;background:radial-gradient(ellipse at 50% 0%,rgba(217,179,92,.16),transparent 60%),#0b0907;color:#f4ecd9;font-family:Vazirmatn,Tahoma,sans-serif;line-height:2}
.c{width:100%;max-width:520px;text-align:center}
.badge{width:116px;height:116px;margin:0 auto 22px;border-radius:50%;display:grid;place-items:center;position:relative;
 background:radial-gradient(circle,rgba(92,191,138,.22),rgba(92,191,138,.04) 70%);border:2px solid rgba(92,191,138,.55)}
.badge.w{background:radial-gradient(circle,rgba(224,168,82,.22),rgba(224,168,82,.04) 70%);border-color:rgba(224,168,82,.6)}
.badge::after{content:'';position:absolute;inset:-10px;border-radius:50%;border:2px solid currentColor;opacity:0;animation:p 2.4s ease-out infinite}
.badge{color:#5cbf8a}.badge.w{color:#e0a852}
@keyframes p{0%{transform:scale(.85);opacity:.6}100%{transform:scale(1.3);opacity:0}}
.badge svg{width:56px;height:56px}
h1{font-size:28px;font-weight:800;color:#f7dfa0;line-height:1.5}
.sub{color:#c2b291;font-size:17px;margin:10px 0 22px}
.card{background:#17120d;border:1px solid rgba(217,179,92,.18);border-radius:16px;padding:16px 18px;text-align:right;margin:12px 0}
.row{display:flex;justify-content:space-between;gap:10px;padding:6px 0;border-bottom:1px dashed rgba(217,179,92,.14)}
.row:last-child{border:0}.row span:first-child{color:#8a7b62}
.ok{color:#5cbf8a;font-weight:700}.warn{color:#e0a852;font-weight:700}
.note{border-inline-start:4px solid #e0a852;background:rgba(224,168,82,.08);border-radius:10px;padding:12px 14px;text-align:right;font-size:15px;margin:12px 0}
a.btn{display:inline-block;margin-top:14px;padding:13px 30px;border-radius:14px;font-weight:800;text-decoration:none;color:#211608;background:linear-gradient(135deg,#f7dfa0,#d9b35c 60%,#9c772d)}
.f{color:#8a7b62;font-size:13px;margin-top:18px}
</style></head><body><main class="c">
<div class="badge${vpn ? ' w' : ''}">${vpn
    ? '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.4" stroke-linecap="round"><path d="M12 8v5M12 16.5v.5"/><circle cx="12" cy="12" r="9"/></svg>'
    : '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.6" stroke-linecap="round" stroke-linejoin="round"><path d="M5 12.5l4.5 4.5L19 7.5"/></svg>'}</div>
${vpn
    ? `<h1>به نظر فیلترشکن روشن است</h1>
<p class="sub">این صفحه باز شد، ولی اتصالِ شما از بیرونِ ایران آمده؛ پس هنوز معلوم نیست بدونِ فیلترشکن هم باز می‌شود یا نه.</p>
<div class="note"><b>فیلترشکن را کامل خاموش کنید</b> و این صفحه را دوباره باز کنید (یا تازه‌سازی کنید). اگر بدونِ فیلترشکن هم همین صفحه آمد، رله روی اینترنتِ شما کار می‌کند.</div>`
    : `<h1>اتصال برقرار است!</h1>
<p class="sub">دامنه‌ی رله‌ها روی اینترنتِ شما باز است؛ یعنی رله‌ی هوش مصنوعی بدونِ فیلترشکن برایتان کار می‌کند.</p>`}
<div class="card">
<div class="row"><span>دامنه‌ی workers.dev</span><span class="ok">در دسترس ✓</span></div>
<div class="row"><span>مرکزِ کلادفلری که جواب داد</span><span>${where}</span></div>
<div class="row"><span>وضعیت</span><span class="${vpn ? 'warn">با فیلترشکن — دوباره بسنجید' : 'ok">آماده‌ی ساختنِ رله'}</span></div>
</div>
<a class="btn" href="${GUIDE}">${vpn ? 'راهنمای ساختنِ رله' : 'برویم رله را بسازیم ←'}</a>
<p class="f">این صفحه چیزی ذخیره نمی‌کند و فقط برای آزمایشِ اتصال است.</p>
</main></body></html>`;
}

export default {
  async fetch(request) {
    const colo = request.cf?.colo || '';
    const country = request.cf?.country || '';
    const cors = { 'access-control-allow-origin': '*', 'cache-control': 'no-store', 'x-robots-tag': 'noindex' };
    if (request.method === 'OPTIONS') return new Response(null, { status: 204, headers: cors });

    // A person opening the address in a browser gets the result page; code (fetch) gets JSON.
    const wantsHtml = (request.headers.get('accept') || '').includes('text/html');
    if (wantsHtml && request.method === 'GET') {
      return new Response(page(colo, country), { headers: { ...cors, 'content-type': 'text/html; charset=utf-8' } });
    }
    return new Response(JSON.stringify({ ok: true, check: 1, colo, country }), {
      headers: { ...cors, 'content-type': 'application/json' },
    });
  },
};
