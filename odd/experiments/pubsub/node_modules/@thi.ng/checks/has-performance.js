import { isFunction } from "./is-function.js";
const hasPerformance = () => typeof performance !== "undefined" && isFunction(performance.now);
export {
  hasPerformance
};
