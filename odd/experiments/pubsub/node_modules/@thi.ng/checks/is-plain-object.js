const OBJP = Object.getPrototypeOf;
const isPlainObject = (x) => {
  let p;
  return x != null && typeof x === "object" && ((p = OBJP(x)) === null || OBJP(p) === null);
};
export {
  isPlainObject
};
