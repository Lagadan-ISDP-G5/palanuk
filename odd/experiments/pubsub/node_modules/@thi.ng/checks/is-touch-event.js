const isTouchEvent = (e) => typeof TouchEvent !== "undefined" && e instanceof TouchEvent;
export {
  isTouchEvent
};
