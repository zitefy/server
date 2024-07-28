const fs = require('fs');
const { JSDOM } = require('jsdom');

function parseHtml(htmlPath, cssPath, jsPath, dataPath) {
  const html = fs.readFileSync(htmlPath, 'utf8');
  const css = fs.readFileSync(cssPath, 'utf8');
  const js = fs.readFileSync(jsPath, 'utf8');
  const data = JSON.parse(fs.readFileSync(dataPath, 'utf8'));

  const dom = new JSDOM(html);
  const document = dom.window.document;

  const style = document.createElement('style');
  style.textContent = css;
  document.head.appendChild(style);

  const script = document.createElement('script');
  script.type = 'module';
  script.textContent = js;
  document.body.appendChild(script);

  data.forEach(item => {
    if (item.selector && item.value) {
      const element = document.querySelector(`#${item.selector}`);
      if (element) {
        let url = item.link && item.value ? `${item.link}${item.value}` : item.link;
        const tagName = element.tagName.toLowerCase();
        let attributeName;
        
        switch (tagName) {
          case 'img':
          case 'video':
          case 'audio':
          case 'source':
          case 'track':
          case 'iframe':
          case 'embed':
          case 'script':
            url = item.value;
            attributeName = 'src';
            break;
          case 'a':
          case 'link':
            attributeName = 'href';
            break;
          default:
            attributeName = 'href';
        }

        element.setAttribute(attributeName, url);

        if (!item.link && !item.value) element.style.display = "none";
        if (element.getAttribute('data-display') === 'true') element.textContent = item.value;
      }
    }
  });

  return dom.serialize();
}

const htmlPath = process.argv[2];
const cssPath = process.argv[3];
const jsPath = process.argv[4];
const dataPath = process.argv[5];

const result = parseHtml(htmlPath, cssPath, jsPath, dataPath);
console.log(result);