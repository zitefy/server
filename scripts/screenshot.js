/*
    This script takes in as input a file that contains a HTML string, renders it in a browser instance, and takes two screenshots in different sizes.
    During initial setup, run bun run puppeteer browsers install chrome, and copy the executable path to line 8
*/
const puppeteer = require('puppeteer');
const fs = require('fs');

(async () => {
    try {
        const browser = await puppeteer.launch({
            executablePath: '/home/vishalds/.cache/puppeteer/chrome/linux-126.0.6478.182/chrome-linux64/chrome',
            args: ['--no-sandbox', '--disable-setuid-sandbox']
        });

        const page = await browser.newPage();
        await page.setRequestInterception(true);

        // Intercept requests to our own API & block them
        // this is a temporary fix, has to be mitigated completely asap
        page.on('request', (request) => {
            if (request.url().includes('api.zitefy.com')) {
                request.abort();
            } else {
                request.continue();
            }
        });

        await page.goto('file://' + process.argv[2], { 
            waitUntil: 'networkidle0', 
            timeout: 60000
        });

        await page.setViewport({ width: 412, height: 915 });
        await page.screenshot({ path: process.argv[3] });

        await page.setViewport({ width: 1280, height: 800 });
        await page.screenshot({ path: process.argv[4] });

        await browser.close();

    } catch (error) {
        fs.writeFileSync('screenshot_error.log', error.toString());
        process.exit(1);
    }
})();