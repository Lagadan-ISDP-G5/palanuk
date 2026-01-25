import { isArray } from "./is-array.js";
import { isString } from "./is-string.js";
const ILLEGAL_KEYS = /* @__PURE__ */ new Set(["__proto__", "prototype", "constructor"]);
const isIllegalKey = (x) => ILLEGAL_KEYS.has(x);
const isProtoPath = (path) => isArray(path) ? path.some(isIllegalKey) : isString(path) ? path.indexOf(".") !== -1 ? path.split(".").some(isIllegalKey) : isIllegalKey(path) : false;
export {
  isIllegalKey,
  isProtoPath
};
