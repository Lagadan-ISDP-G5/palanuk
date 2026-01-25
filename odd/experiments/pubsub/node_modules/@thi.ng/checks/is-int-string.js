const RE = /^(?:[-+]?(?:0|[1-9][0-9]*))$/;
const isIntString = (x) => RE.test(x);
export {
  isIntString
};
