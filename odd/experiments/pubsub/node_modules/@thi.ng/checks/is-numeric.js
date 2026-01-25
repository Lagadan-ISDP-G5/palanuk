const isNumericInt = (x) => /^[-+]?\d+$/.test(x);
const isNumericFloat = (x) => /^[-+]?\d*\.?\d+(e[-+]?\d+)?$/i.test(x);
export {
  isNumericFloat,
  isNumericInt
};
