const hasCrypto = () => typeof self !== "undefined" && self["crypto"] !== void 0 || typeof global !== "undefined" && global["crypto"] !== void 0;
export {
  hasCrypto
};
