const HOST_NAME = "com.alvenqis.browser_host";

let nextId = 1;

chrome.runtime.onMessage.addListener((message, _sender, sendResponse) => {
  if (!message || message.type !== "host") {
    return false;
  }

  const id = nextId++;
  const request = {
    id,
    method: message.method,
    params: message.params || {},
  };

  try {
    chrome.runtime.sendNativeMessage(HOST_NAME, request, (response) => {
      const error = chrome.runtime.lastError;
      if (error) {
        sendResponse({
          ok: false,
          error: error.message || String(error),
        });
        return;
      }
      sendResponse(response || { ok: false, error: "empty host response" });
    });
  } catch (error) {
    sendResponse({
      ok: false,
      error: error instanceof Error ? error.message : String(error),
    });
  }

  return true;
});
