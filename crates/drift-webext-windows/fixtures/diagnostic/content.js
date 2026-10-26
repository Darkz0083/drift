document.documentElement.dataset.driftExtension = chrome.runtime.id;
chrome.runtime.sendMessage({op: "ping"}).then(result => {
  document.documentElement.dataset.driftWorker = JSON.stringify(result);
}).catch(error => {
  document.documentElement.dataset.driftWorkerError = String(error);
});
