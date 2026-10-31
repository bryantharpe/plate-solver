// Real session writes belong on the session project's separate server. Oracles: the
// FR-033 phone layout, the saved M42 suggestions, and DESIGN.md's #e00000 night ink.
import { expect, test, type Locator, type Page } from "@playwright/test";
import type { SimilarReport } from "../src/api";
import similarM42 from "../src/test/fixtures/similar-m42.json" with { type: "json" };

test.beforeEach(async ({ request }) => {
  const response = await request.put("/api/session", {
    data: { site: null, min_alt_deg: 15, target: null },
  });
  expect(response.ok()).toBe(true);
});

async function openConsole(page: Page) {
  await page.addInitScript(() => {
    localStorage.setItem("finder.frame", JSON.stringify("orion2.jpg"));
    localStorage.setItem("finder.night", "false");
  });
  await page.goto("/");
  await expect(page.getByRole("heading", { name: "Orion", exact: true })).toBeVisible({ timeout: 30_000 });
  await page.evaluate(() => document.fonts.ready);
}

async function chooseM78(page: Page) {
  await page.getByRole("button", { name: "Choose target", exact: true }).click();
  const sheet = page.getByRole("dialog", { name: "Choose a target" });
  await sheet.getByRole("searchbox", { name: "Search catalog" }).fill("M78");
  await sheet
    .getByRole("list", { name: "Matching targets" })
    .getByRole("button", { name: /\(M78\)/ })
    .click();
  await expect(sheet).not.toBeVisible();
  await expect(page.getByRole("region", { name: /^Target M78/ })).toBeVisible();
  // The marker also proves this page has received the latest pointing's session poll.
  await expect(page.locator('[data-target="M78"]')).toBeAttached({ timeout: 10_000 });
}

async function box(locator: Locator) {
  const bounds = await locator.boundingBox();
  expect(bounds).not.toBeNull();
  return bounds!;
}

/** Any green or blue is forbidden; red may exceed ink (224) by only two rounding levels. */
async function offRed(page: Page, png: Buffer): Promise<number> {
  return page.evaluate(async (data) => {
    const image = new Image();
    image.src = `data:image/png;base64,${data}`;
    await image.decode();
    const canvas = document.createElement("canvas");
    canvas.width = image.width;
    canvas.height = image.height;
    const context = canvas.getContext("2d")!;
    context.drawImage(image, 0, 0);
    const pixels = context.getImageData(0, 0, canvas.width, canvas.height).data;
    let off = 0;
    for (let i = 0; i < pixels.length; i += 4) {
      if (pixels[i]! > 226 || pixels[i + 1]! > 0 || pixels[i + 2]! > 0) off++;
    }
    return off;
  }, png.toString("base64"));
}

async function enableNight(page: Page) {
  const toggle = page.getByRole("switch", { name: "Night mode: red and black" });
  await toggle.click();
  await expect(toggle).toBeChecked();
  await expect(page.locator("html")).toHaveAttribute("data-theme", "night");
}

for (const [width, height] of [
  [412, 915],
  [390, 844],
] as const) {
  test.describe(`with a target on a phone, ${width} × ${height}`, () => {
    test.use({ viewport: { width, height }, deviceScaleFactor: 2 });

    test("guidance, title, chart and legend fit the first screen, with the roster after them", async ({ page }) => {
      await openConsole(page);
      await chooseM78(page);
      // Choosing a target can scroll the dialog; measure the actual first screen.
      await page.evaluate(() => window.scrollTo(0, 0));
      const guidance = page.getByRole("region", { name: /^Target M78/ });
      const title = page.getByRole("region", { name: "Orion", exact: true });
      const chart = page.getByRole("img", { name: /^Chart of/ });
      const legend = page.getByRole("group", { name: "Show on the chart" });
      let bottom = 0;
      for (const locator of [guidance, title, chart, legend]) {
        await expect(locator).toBeInViewport({ ratio: 1 });
        const bounds = await box(locator);
        // CSS-pixel bounds, with no clipping or overlap tolerance (FR-033).
        expect(bounds.width).toBeGreaterThan(0);
        expect(bounds.height).toBeGreaterThan(0);
        expect(bounds.x).toBeGreaterThanOrEqual(0);
        expect(bounds.x + bounds.width).toBeLessThanOrEqual(width);
        expect(bounds.y).toBeGreaterThanOrEqual(bottom);
        bottom = bounds.y + bounds.height;
        expect(bottom).toBeLessThanOrEqual(height);
      }
      const roster = page.getByText("Alnilam, Alnitak, Mintaka", { exact: true }).first().locator("xpath=../..");
      expect((await box(roster)).y).toBeGreaterThan(bottom);
    });
  });
}

test("a site saved on the phone reaches a separate laptop and ranks its suggestions", async ({
  page,
  browser,
  baseURL,
}) => {
  await page.setViewportSize({ width: 412, height: 915 });
  await page.goto("/setup");
  await expect(page.getByRole("heading", { name: "Setup", exact: true })).toBeVisible();
  await page.getByRole("textbox", { name: "Latitude", exact: true }).fill("40");
  await page.getByRole("textbox", { name: "Longitude", exact: true }).fill("-75");
  await page.getByRole("button", { name: "Save site", exact: true }).click();
  await expect(page.getByRole("status")).toHaveText("Site saved.");

  const laptop = await browser.newContext({ baseURL, viewport: { width: 1440, height: 900 } });
  try {
    const other = await laptop.newPage();
    // Only the browser's explicit-site request clock is fixed. The real server keeps
    // its system clock. This saved capture is for 40 N, 75 W on 2026-01-16 at 02:00 UT.
    await other.clock.setFixedTime(new Date(similarM42.observer.time * 1000));
    const response = other.waitForResponse((r) => {
      const url = new URL(r.url());
      return url.pathname === "/api/similar" && url.searchParams.get("lat") === "40";
    });
    await openConsole(other);
    await expect(other.getByRole("button", { name: "Site 40.0° N", exact: true })).toBeVisible();
    expect(await other.evaluate(() => localStorage.getItem("finder.site"))).toBeNull();

    const ranked = await response;
    expect(ranked.ok()).toBe(true);
    expect(new URL(ranked.url()).searchParams.get("lon")).toBe("-75");
    const report = (await ranked.json()) as SimilarReport;
    expect(report.observer).toEqual(similarM42.observer);
    expect(report.reference.id).toBe("M42");
    // The saved capture requested four rows. The page requests six, adding M16,
    // the remaining diffuse nebula in the pinned Messier catalog.
    const expectedIds = [...similarM42.suggestions.map((s) => s.id), "M16"];
    expect(report.suggestions.map((s) => s.id)).toEqual(expectedIds);
    for (const [i, saved] of similarM42.suggestions.entries()) {
      expect(report.suggestions[i]!.up).toBe(saved.up);
      expect(report.suggestions[i]!.alt_deg).toBeCloseTo(saved.alt_deg, 6); // saved capture, ±0.000001°
    }
    const suggestions = other
      .getByRole("heading", { name: "Like M42: other diffuse nebulae", exact: true })
      .locator("..");
    const rows = suggestions.getByRole("listitem");
    await expect(rows).toHaveCount(expectedIds.length);
    for (const [i, id] of expectedIds.entries()) {
      await expect(rows.nth(i).locator("span.italic")).toHaveText(id);
      await expect(rows.nth(i)).toContainText(i === 0 ? "47° up" : "below 15°");
    }
    await expect(suggestions.getByRole("button", { name: "Set your site" })).toHaveCount(0);

    // The second device reads both coordinates from the shared observing session.
    await other.getByRole("link", { name: "Setup", exact: true }).click();
    await expect(other.getByRole("textbox", { name: "Latitude", exact: true })).toHaveValue("40");
    await expect(other.getByRole("textbox", { name: "Longitude", exact: true })).toHaveValue("-75");
  } finally {
    await laptop.close();
  }
});

test("opening /setup directly renders Setup through the server's page fallback", async ({ page }) => {
  const response = await page.goto("/setup");
  expect(response?.status()).toBe(200);
  expect(response?.headers()["content-type"]).toContain("text/html");
  await expect(page).toHaveURL(/\/setup$/);
  await expect(page.getByRole("heading", { name: "Setup", exact: true })).toBeVisible();
  await expect(page.getByRole("textbox", { name: "Latitude", exact: true })).toBeVisible();
  await expect(page.getByRole("textbox", { name: "Longitude", exact: true })).toBeVisible();
  await expect(page.getByRole("textbox", { name: "Minimum altitude", exact: true })).toHaveValue("15");
  await expect(page.getByRole("link", { name: "Back to console" })).toBeVisible();
});

test.describe("red-only night on a phone", () => {
  test.use({ viewport: { width: 412, height: 915 }, deviceScaleFactor: 2 });

  test("orion2 with M78 as the target has no off-red pixel", async ({ page }) => {
    await openConsole(page);
    await chooseM78(page);
    // A daylight screenshot is the negative control for this same pixel scanner.
    expect(await offRed(page, await page.screenshot({ fullPage: true }))).toBeGreaterThan(100_000);
    await enableNight(page);
    expect(await offRed(page, await page.screenshot({ fullPage: true, animations: "disabled" }))).toBe(0);
  });

  test("Setup has no off-red pixel", async ({ page }) => {
    await page.goto("/setup");
    await expect(page.getByRole("heading", { name: "Setup", exact: true })).toBeVisible();
    await page.evaluate(() => document.fonts.ready);
    expect(await offRed(page, await page.screenshot({ fullPage: true }))).toBeGreaterThan(100_000);
    await enableNight(page);
    expect(await offRed(page, await page.screenshot({ fullPage: true, animations: "disabled" }))).toBe(0);
  });
});
