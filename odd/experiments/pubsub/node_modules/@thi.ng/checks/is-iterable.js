const isIterable = (x) => typeof x?.[Symbol.iterator] === "function";
export {
  isIterable
};
