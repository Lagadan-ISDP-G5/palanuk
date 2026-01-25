const isPositive = (x) => typeof x === "number" && x > 0;
const isPositiveInt = (x) => Number.isInteger(x) && x > 0;
const isNonNegativeInt = (x) => Number.isInteger(x) && x >= 0;
export {
  isNonNegativeInt,
  isPositive,
  isPositiveInt
};
