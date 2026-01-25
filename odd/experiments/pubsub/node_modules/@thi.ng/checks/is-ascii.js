const isASCII = (x) => /^[\x00-\x7f]+$/.test(x);
const isPrintableASCII = (x) => /^[\x20-\x7e]+$/.test(x);
export {
  isASCII,
  isPrintableASCII
};
