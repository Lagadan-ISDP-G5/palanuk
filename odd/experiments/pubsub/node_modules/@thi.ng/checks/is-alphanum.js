const isAlpha = (x) => /^[a-z]+$/i.test(x);
const isAlphaNum = (x) => /^[a-z0-9]+$/i.test(x);
const isNumeric = (x) => /^[0-9]+$/.test(x);
export {
  isAlpha,
  isAlphaNum,
  isNumeric
};
