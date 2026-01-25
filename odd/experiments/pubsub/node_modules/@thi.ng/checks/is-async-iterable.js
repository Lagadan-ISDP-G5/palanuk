const isAsyncIterable = (x) => typeof x?.[Symbol.asyncIterator] === "function";
export {
  isAsyncIterable
};
