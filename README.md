# AI Relay

A tiny open-source tool that builds a **private relay for AI APIs** (Gemini, OpenAI, Claude, OpenRouter, Groq, xAI, Mistral, DeepSeek) on **your own free Cloudflare account**. Apps that let you set a custom API base URL can then reach these services through the relay.

<div dir="rtl">

## فارسی

ابزارِ کوچک و متن‌باز برای ساختنِ **رله‌ی شخصیِ هوش مصنوعی** روی **حسابِ رایگانِ کلادفلرِ خودتان**؛ تا برنامه‌هایی که «آدرسِ سفارشی» قبول می‌کنند، بدونِ فیلترشکن به Gemini، OpenAI، Claude و… وصل شوند.

**چطور کار می‌کند:** فایل را اجرا کنید؛ یک صفحه‌ی فارسی در مرورگرتان باز می‌شود (روی `127.0.0.1`، یعنی روی همین کامپیوتر). توکنِ کلادفلر را می‌دهید و ابزار ورکر را روی حسابتان می‌سازد و آدرس‌های آماده را نشان می‌دهد.

**حریم و امنیت:**
- توکن **فقط** به `api.cloudflare.com` فرستاده می‌شود. هیچ سرورِ دیگری در کار نیست و چیزی روی دیسک ذخیره نمی‌شود.
- صفحه‌ی محلی با یک کلیدِ تصادفیِ همان اجرا محافظت می‌شود؛ سایت‌های دیگر نمی‌توانند از آن استفاده کنند.
- ورکر فقط به سرویس‌های هوش مصنوعیِ بالا راه می‌دهد (پراکسیِ باز نیست) و بدونِ کلیدِ مخفی فقط 404 نشان می‌دهد.
- کلِ کد همین مخزن است: [`src/main.rs`](src/main.rs)، [`ui/index.html`](ui/index.html)، [`worker/worker.js`](worker/worker.js).

**هشدارِ ویندوز:** فایل امضای تجاریِ مایکروسافت (Authenticode) ندارد، پس ممکن است پیامِ «Windows protected your PC» ببینید: `More info` ← `Run anyway`. به‌جای آن، اصالتِ فایل را با گیت‌هاب بسنجید (پایین).

**دسترسی‌های توکن:** `Account › Workers Scripts › Edit` و `Account › Account Settings › Read`. بعد از ساختنِ رله، توکن را پاک کنید.

</div>

## Verify a download

Every release file is built by GitHub Actions from this repository and carries a signed build-provenance attestation:

```bash
gh attestation verify ai-relay-windows-x64.exe --repo <owner>/<repo>
```

or compare against `SHA256SUMS.txt` in the release:

```powershell
Get-FileHash .\ai-relay-windows-x64.exe -Algorithm SHA256
```

## Token permissions

| Scope | Permission | Why |
|---|---|---|
| Account | Workers Scripts: Edit | create/delete the worker and the workers.dev subdomain |
| Account | Account Settings: Read | list your accounts (optional: you can type the Account ID) |

Delete the token after setup.

## Relay addresses

`https://<worker>.<subdomain>.workers.dev/<SECRET>/<service>/<path>`

| Service | API format | Base URL |
|---|---|---|
| Gemini | Gemini | `…/<SECRET>/gemini/v1beta` |
| OpenAI | OpenAI | `…/<SECRET>/openai/v1` |
| Anthropic | Anthropic | `…/<SECRET>/anthropic/v1` |
| OpenRouter | OpenAI | `…/<SECRET>/openrouter/api/v1` |
| Groq | OpenAI | `…/<SECRET>/groq/openai/v1` |
| xAI / Mistral / DeepSeek | OpenAI | `…/<SECRET>/xai/v1`, `mistral/v1`, `deepseek/v1` |

Health check: `…/<SECRET>/ping`.

**All addresses at once:** open `https://<worker>.<subdomain>.workers.dev/<SECRET>/` in a browser (any address with the service part removed). It lists every service with a copy button; bookmark it.

**Saved on this computer:** after a successful build the tool keeps the relay address and secret in `%APPDATA%\ai-relay\relays.json` (Windows) or `~/.config/ai-relay/relays.json`, and shows them the next time you run it. The Cloudflare token is never stored. You can also paste any old address to rebuild the full list.

## Manual setup (no tool)

1. Cloudflare dashboard → Workers & Pages → Create → Hello World → Deploy.
2. Edit code → replace with [`worker/worker.js`](worker/worker.js) → Deploy.
3. Settings → Variables and Secrets → Add → type **Secret**, name `SECRET`, a random value of 16+ characters.

## Build

```bash
cargo build --release
```

Set `AI_RELAY_NO_BROWSER=1` to skip opening the browser.

The UI font is not part of this repository. Release builds add it from the repository secrets `FONT_REGULAR`, `FONT_BOLD` and `FONT_EXTRABOLD` (base64 of the `.woff2` files); local builds without a `fonts/` folder use the system font.

## License

MIT
