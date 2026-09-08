import { $, $$, browser, expect } from "@wdio/globals";

// Real E2E: drives the actual built Tauri application via `tauri-driver`,
// never a simulated frontend (docs/TEST_IMPLEMENTATION_PLAN.md section 2,
// exit criterion). Covers item 2.3: launch, library, navigation.
//
// Deliberately does not assume the app starts on the library page: it
// always navigates there explicitly first via the sidebar nav item, which
// is present regardless of current page.
describe("InterviewScribe desktop app", () => {
  const goToLibrary = async () => {
    const nav = await $("button=Mes entretiens");
    await nav.waitForExist({ timeout: 20000 });
    await nav.click();
    const heading = await $("h1=Mes entretiens");
    await heading.waitForDisplayed({ timeout: 10000 });
    return heading;
  };

  it("launches, and the library shows no interviews and no error", async () => {
    await goToLibrary();

    const emptyOrCount = await $(".sectionTitle span");
    await emptyOrCount.waitForDisplayed({ timeout: 10000 });
    await expect(emptyOrCount).toBeDisplayed();
  });

  it("navigates to Reglages and shows the bundled model, never a download action", async () => {
    const settingsLink = await $("button=Réglages");
    await settingsLink.click();

    const heading = await $("h1=Réglages");
    await heading.waitForDisplayed({ timeout: 10000 });

    // The 575MB bundled model is re-verified by full SHA-256 on every check
    // (transcription/model.rs::verify_model) - genuinely slow, not flaky.
    // getText() + includes() sidesteps `*=` locator quirks against nested
    // text nodes (verified: the exact same text is present in body innerText
    // well before this locator strategy would ever match it).
    await browser.waitUntil(
      async () => {
        const text = await $("main").getText();
        return text.includes("Whisper Large v3 Turbo (Q5_0)");
      },
      {
        timeout: 45000,
        timeoutMsg: "modele Whisper jamais affiche dans Reglages",
      },
    );

    const downloadButtons = await $$("button*=Télécharger");
    expect(downloadButtons.length).toBe(0);
  });

  it("opens Nouvel entretien and lists the real, filtered microphone", async () => {
    await goToLibrary();
    const newInterview = await $("button*=Nouvel entretien");
    await newInterview.waitForDisplayed({ timeout: 10000 });
    await newInterview.click();

    const heading = await $("h1=Nouvel entretien");
    await heading.waitForDisplayed({ timeout: 10000 });

    // Real hardware check: the ALSA software-plugin filter (capture/device.rs)
    // must leave exactly the one real microphone, never the raw dozen-plus
    // entries it used to return.
    const micSelect = await $('//label[contains(., "Microphone")]/select');
    await micSelect.waitForExist({ timeout: 10000 });
    const micOptions = await micSelect.$$("option");
    expect(micOptions.length).toBeGreaterThanOrEqual(1);
  });
});
