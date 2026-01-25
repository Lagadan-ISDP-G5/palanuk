const hasWASM = () => typeof self !== "undefined" && typeof self["WebAssembly"] !== "undefined" || typeof global !== "undefined" && typeof global["WebAssembly"] !== "undefined";
export {
  hasWASM
};
