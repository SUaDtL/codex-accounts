// ruleid: renderer-code-execution
eval(input);
// ruleid: renderer-code-execution
new Function(input);
// ok: renderer-code-execution
JSON.parse(input);
// ruleid: renderer-html-injection
element.innerHTML = input;
// ruleid: renderer-html-injection
element.insertAdjacentHTML("beforeend", input);
// ok: renderer-html-injection
element.textContent = input;
// ruleid: renderer-persistent-storage
localStorage.setItem("account", input);
// ok: renderer-persistent-storage
accounts.set("account", input);
