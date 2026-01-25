const isTypedArray = (x) => !!x && (x instanceof Uint8Array || x instanceof Int8Array || x instanceof Uint16Array || x instanceof Int16Array || x instanceof Uint32Array || x instanceof Int32Array || x instanceof Float32Array || x instanceof Float64Array || x instanceof Uint8ClampedArray);
const isIntArray = (x) => !!x && (x instanceof Int8Array || x instanceof Int16Array || x instanceof Int32Array);
const isUintArray = (x) => !!x && (x instanceof Uint32Array || x instanceof Uint16Array || x instanceof Uint8Array || x instanceof Uint8ClampedArray);
const isFloatArray = (x) => !!x && (x instanceof Float32Array || x instanceof Float64Array);
export {
  isFloatArray,
  isIntArray,
  isTypedArray,
  isUintArray
};
