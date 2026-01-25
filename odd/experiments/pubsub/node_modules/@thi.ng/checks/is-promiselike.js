import { implementsFunction } from "./implements-function.js";
const isPromiseLike = (x) => x instanceof Promise || implementsFunction(x, "then") && implementsFunction(x, "catch");
export {
  isPromiseLike
};
