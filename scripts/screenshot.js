/*
    This script takes in as input a file that contains a HTML string, renders it in a browser instance, and takes two screenshots in different sizes.
    During initial setup, run bun run puppeteer browsers install chrome, and copy the executable path to line 8
*/
const puppeteer = require('puppeteer');

(async () => {
    const browser = await puppeteer.launch({executablePath: '/home/vishalds/.cache/puppeteer/chrome/linux-126.0.6478.126/chrome-linux64/chrome'});
    const page = await browser.newPage();
    await page.goto('file://' + process.argv[2]);

    // Mobile screenshot
    await page.setViewport({ width: 412, height: 915 });
    await page.screenshot({ path: process.argv[3] });

    // Desktop screenshot
    await page.setViewport({ width: 1280, height: 800 });
    await page.screenshot({ path: process.argv[4] });

    await browser.close();
})();