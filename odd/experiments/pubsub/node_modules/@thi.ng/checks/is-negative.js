const isNegative = (x) => typeof x === "number" && x < 0;
const isNegativeInt = (x) => Number.isInteger(x) && x < 0;
const isNonPositiveInt = (x) => Number.isInteger(x) && x <= 0;
export {
  isNegative,
  isNegativeInt,
  isNonPositiveInt
};
