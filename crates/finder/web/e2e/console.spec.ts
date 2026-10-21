// The console in a real browser against the real server: what the phone's first screen
// shows, that night mode puts nothing on screen but red, the desktop layout, working
// offline, and the paths a frame takes (a sample, an upload, a frame that does not solve).

import { expect, test, type Locator, type Page } from "@playwright/test";

interface Visit {
  frame?: string;
  night?: boolean;
  site?: boolean;
}

/** Open the console as a returning user: night mode, site and last frame from storage. */
async function open(page: Page, { frame, night = false, site = false }: Visit = {}) {
  await page.addInitScript(
    ([frame, night, site]) => {
      localStorage.setItem("finder.night", JSON.stringify(night));
      if (frame) localStorage.setItem("finder.frame", JSON.stringify(frame));
      if (site) localStorage.setItem("finder.site", JSON.stringify({ lat: 40, lon: -75 }));
    },
    [frame, night, site] as const,
  );
  await page.goto("/");
  if (frame) await expect(page.getByText(/Solved in|No match/)).toBeVisible({ timeout: 30_000 });
  await page.evaluate(() => document.fonts.ready);
}

const box = async (l: Locator) => {
  const b = await l.boundingBox();
  expect(b).not.toBeNull();
  return b as NonNullable<typeof b>;
};

/**
 * Pixels in a screenshot that break night mode: any green or blue, or red brighter than the
 * night ink (#e00000, 224). Anti-aliased ink over a bright star can composite one level
 * over, so two levels of rounding are allowed.
 */
async function offRed(page: Page, png: Buffer): Promise<number> {
  return page.evaluate(async (data) => {
    const img = new Image();
    img.src = `data:image/png;base64,${data}`;
    await img.decode();
    const canvas = document.createElement("canvas");
    canvas.width = img.width;
    canvas.height = img.height;
    const g = canvas.getContext("2d") as CanvasRenderingContext2D;
    g.drawImage(img, 0, 0);
    const d = g.getImageData(0, 0, canvas.width, canvas.height).data;
    let off = 0;
    for (let i = 0; i < d.length; i += 4) {
      if ((d[i] ?? 0) > 226 || (d[i + 1] ?? 0) > 0 || (d[i + 2] ?? 0) > 0) off++;
    }
    return off;
  }, png.toString("base64"));
}

for (const [width, height] of [
  [412, 915],
  [390, 844],
] as const) {
  test.describe(`on a phone, ${width} × ${height}`, () => {
    test.use({ viewport: { width, height }, deviceScaleFactor: 2 });

    test("the first screen shows where it points, what is in it, and the whole chart with its legend", async ({
      page,
    }) => {
      await open(page, { frame: "orion2.jpg", site: true });
      const card = page.getByRole("region", { name: "Orion" });
      const roster = page.getByText("Alnilam, Alnitak, Mintaka").first();
      const chart = page.getByRole("img", { name: /^Chart of/ });
      const legend = page.getByRole("group", { name: "Show on the chart" });
      for (const l of [card, roster, chart, legend]) await expect(l).toBeInViewport({ ratio: 1 });
      const tops = await Promise.all([card, roster, chart, legend].map(async (l) => (await box(l)).y));
      expect(tops).toEqual([...tops].sort((a, b) => a - b));
    });
  });
}

test.describe("on the user's phone, 412 × 915", () => {
  test.use({ viewport: { width: 412, height: 915 }, deviceScaleFactor: 2 });

  test("night mode puts nothing on screen but red, none brighter than the ink: chart, lists, picker", async ({
    page,
  }) => {
    // The check can fail: the day edition, with its paper and photographs, is far from red.
    await open(page, { frame: "orion2.jpg" });
    expect(await offRed(page, await page.screenshot())).toBeGreaterThan(100_000);

    for (const frame of ["orion2.jpg", "virgo.jpg"]) {
      await open(page, { frame, night: true, site: true });
      await page.waitForTimeout(1500); // the chart inks in
      expect(await offRed(page, await page.screenshot({ fullPage: true })), frame).toBe(0);
    }
    await page.getByRole("button", { name: "virgo.jpg" }).click();
    await expect(page.getByRole("dialog")).toBeVisible();
    await page.waitForTimeout(800); // thumbnails
    expect(await offRed(page, await page.screenshot()), "frame picker").toBe(0);
  });
});

test.describe("on a laptop, 1440 × 900", () => {
  test.use({ viewport: { width: 1440, height: 900 } });

  test("the chart sits beside the facts, stays in view, and the page keeps even margins", async ({ page }) => {
    await open(page, { frame: "orion2.jpg", site: true });
    const card = await box(page.getByRole("region", { name: "Orion" }));
    const chart = page.locator("figure.chart");
    const c = await box(chart);
    expect(c.x).toBeGreaterThan(card.x + card.width);
    expect(card.x).toBeGreaterThanOrEqual(40);
    const right = 1440 - (c.x + c.width);
    expect(right).toBeGreaterThanOrEqual(40);
    expect(Math.abs(right - card.x)).toBeLessThan(32);
    await expect(chart).toBeInViewport({ ratio: 1 });

    // Reading on down the left column, the chart stays put.
    await page.mouse.wheel(0, 500);
    await expect(page.getByRole("heading", { name: "What to look at next" })).toBeInViewport();
    await expect(page.getByRole("img", { name: /^Chart of/ })).toBeInViewport({ ratio: 1 });
  });
});

test("works offline: nothing comes from another host until the survey is asked for", async ({ page, baseURL }) => {
  const hosts = new Set<string>();
  page.on("request", (r) => {
    if (r.url().startsWith("http")) hosts.add(new URL(r.url()).host);
  });
  await page.route(/aladin\.cds\.unistra\.fr/, (route) => route.abort());
  await open(page, { frame: "orion2.jpg", site: true });
  await page.waitForLoadState("networkidle");
  expect([...hosts]).toEqual([new URL(baseURL ?? "").host]);

  await page.getByRole("button", { name: "Show the survey image" }).click();
  await expect(page.getByText(/No connection to the survey/)).toBeVisible();
  expect(hosts.has("aladin.cds.unistra.fr")).toBe(true);
});

test("a frame from the device solves and is charted", async ({ page }) => {
  await open(page);
  await page.getByRole("button", { name: "Open your own frame" }).click();
  await page
    .locator('input[type="file"]')
    .setInputFiles("../../../reference-solutions/cedar-solve/examples/data/medium_fov/pleiades.jpg");
  await expect(page.getByRole("heading", { name: "Taurus" })).toBeVisible({ timeout: 30_000 });
  await expect(page.locator('[data-object="M45"]')).toBeAttached();
});

test("a frame that does not solve says what to try, and solves at the lower threshold it offers", async ({ page }) => {
  await open(page, { frame: "roof.jpg" });
  await expect(page.getByRole("heading", { name: "No match" })).toBeVisible();
  const retry = page.waitForRequest(
    (r) => r.url().endsWith("/api/solve") && /name="sigma"\r\n\r\n5\r\n/.test(r.postData() ?? ""),
  );
  await page.getByRole("button", { name: "Try again at 5σ" }).click();
  await retry;
  // The roof frame's stars are faint: at 5σ there are enough of them to solve.
  await expect(page.getByText(/Solved in/)).toBeVisible({ timeout: 30_000 });
});
