// Layout sanity check — Vite dev URL을 띄워 .terminal__stream의 스크롤 동작 측정.
// 사용 시스템 Edge 채널 (chromium download 회피).
import { chromium } from "playwright";

const URL = "http://localhost:1420";

const browser = await chromium.launch({ channel: "msedge", headless: true });
const ctx = await browser.newContext({ viewport: { width: 1280, height: 800 } });
const page = await ctx.newPage();

await page.goto(URL, { waitUntil: "domcontentloaded" });
await page.waitForSelector(".terminal__stream", { timeout: 10000 });

// 강제로 가짜 블록 30개 주입 (markdown 렌더 흉내, plain HTML)
await page.evaluate(() => {
  const stream = document.querySelector(".terminal__stream");
  if (!stream) return;
  for (let i = 0; i < 30; i++) {
    const block = document.createElement("div");
    block.className = "block block--assistant";
    block.innerHTML = `
      <div class="block__header"><span class="block__role-dot"></span><span>ai</span></div>
      <div class="block__body">테스트 블록 ${i}: 한국어 길고 긴 문장. ${"가나다라마바사아자차카타파하 ".repeat(15)}</div>
    `;
    stream.appendChild(block);
  }
});

await page.waitForTimeout(300);

const m = await page.evaluate(() => {
  const q = (sel) => document.querySelector(sel);
  const measure = (el) => el ? {
    offsetHeight: el.offsetHeight,
    scrollHeight: el.scrollHeight,
    clientHeight: el.clientHeight,
    scrollTop: el.scrollTop,
    rect: el.getBoundingClientRect(),
  } : null;
  return {
    viewport: { w: window.innerWidth, h: window.innerHeight },
    app: measure(q(".app")),
    main: measure(q(".app__main")),
    terminal: measure(q(".terminal")),
    stream: measure(q(".terminal__stream")),
    input: measure(q(".terminal__input")),
    statusbar: measure(q(".statusbar")),
    canScroll: (() => {
      const s = q(".terminal__stream");
      return s ? s.scrollHeight > s.clientHeight : false;
    })(),
  };
});

console.log(JSON.stringify(m, null, 2));

// 스크롤을 끝으로 이동
await page.evaluate(() => {
  const s = document.querySelector(".terminal__stream");
  if (s) s.scrollTop = s.scrollHeight;
});
await page.waitForTimeout(200);
const after = await page.evaluate(() => {
  const s = document.querySelector(".terminal__stream");
  return s ? { scrollTop: s.scrollTop, scrollHeight: s.scrollHeight, clientHeight: s.clientHeight, atBottom: s.scrollHeight - s.clientHeight - s.scrollTop < 2 } : null;
});
console.log("AFTER scroll:", JSON.stringify(after, null, 2));

await page.screenshot({ path: "scripts/layout-check.png", fullPage: false });

await browser.close();
console.log("\nScreenshot: scripts/layout-check.png");
