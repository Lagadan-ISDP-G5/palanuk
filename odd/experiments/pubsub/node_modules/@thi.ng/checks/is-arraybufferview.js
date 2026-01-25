import { isArrayBufferLike } from "./is-arraybufferlike.js";
import { isNumber } from "./is-number.js";
const isArrayBufferView = (x) => x != null && isArrayBufferLike(x.buffer) && isNumber(x.byteOffset) && isNumber(x.byteLength);
export {
  isArrayBufferView
};
