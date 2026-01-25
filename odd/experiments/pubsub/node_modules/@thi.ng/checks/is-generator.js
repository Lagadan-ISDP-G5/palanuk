import { implementsFunction } from "./implements-function.js";
import { isIterable } from "./is-iterable.js";
const isGenerator = (x) => isIterable(x) && implementsFunction(x, "next");
export {
  isGenerator
};
