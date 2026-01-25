const isPrimitive = (x) => {
  const t = typeof x;
  return t === "string" || t === "number" || t === "boolean";
};
export {
  isPrimitive
};
