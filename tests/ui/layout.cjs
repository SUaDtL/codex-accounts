const {chromium}=require('playwright');
const fs=require('fs'),path=require('path'),assert=require('assert');
const root=process.cwd(),out=path.join(root,'reports/ui/layout');fs.mkdirSync(out,{recursive:true});
(async()=>{const browser=await chromium.launch({headless:true});try{
 const page=await browser.newPage({viewport:{width:940,height:360}});const errors=[];page.on('pageerror',e=>errors.push(e.message));
 await page.route('**/*',async r=>{const name=new URL(r.request().url()).pathname.slice(1)||'index.html';if(!['index.html','app.css','app.js','app-icon.png'].includes(name))return r.abort();await r.fulfill({body:fs.readFileSync(path.join(root,'crates/desktop-app/assets',name)),contentType:name.endsWith('html')?'text/html':name.endsWith('css')?'text/css':name.endsWith('js')?'application/javascript':'image/png'});});
 await page.addInitScript(()=>{window.calls=[];window.fixture={state:'unopened',vault_exists:true,profiles:[],operations:[],recovery:'none'};window.__TAURI__={core:{invoke:async(command,args)=>{calls.push({command,args});if(command==='read_status')return {inventory:{state:'observed',installations:[{version:[26,1,0,0],architecture:'x64'}]}};if(command==='open_vault')return new Promise((resolve,reject)=>{window.finishOpen=()=>resolve(structuredClone(fixture));window.failOpen=()=>reject('E_PATH_UNSAFE');});return structuredClone(fixture);}}};});
 await page.goto('http://ui.localhost/');await page.locator('#setup-button:not([disabled])').waitFor();
 await page.locator('#setup-button').click();await page.waitForFunction(()=>typeof finishOpen==='function');
 await page.screenshot({path:path.join(out,'opening.png')});
 const fits=()=>page.evaluate(()=>({height:innerHeight,scroll:document.documentElement.scrollHeight,statusBottom:document.getElementById('activity').getBoundingClientRect().bottom}));
 let bounds=await fits();assert(bounds.scroll<=bounds.height && bounds.statusBottom<=bounds.height,`Pending vault open clips status or scrolls: ${JSON.stringify(bounds)}`);
 for(const size of [{width:940,height:328},{width:420,height:320}]){await page.setViewportSize(size);bounds=await fits();assert(bounds.scroll<=bounds.height && bounds.statusBottom<=bounds.height,`Pending status does not fit: ${JSON.stringify(bounds)}`);}
 await page.evaluate(()=>failOpen());await page.locator('#setup-button:not([disabled])').waitFor();bounds=await fits();assert(bounds.scroll<=bounds.height && bounds.statusBottom<=bounds.height,'Wrapped failure must fit');
 await page.screenshot({path:path.join(out,'failure-narrow.png')});assert.deepEqual(errors,[]);
 console.log('PASS delayed Open vault and wrapped failure at desktop and minimum window sizes. Synthetic IPC only.');
}finally{await browser.close();}})().catch(e=>{console.error(e);process.exit(1)});
