import { isChrome } from "./is-chrome.js";
const isSafari = () => typeof navigator !== "undefined" && /Safari/.test(navigator.userAgent) && !isChrome();
export {
  isSafari
};
