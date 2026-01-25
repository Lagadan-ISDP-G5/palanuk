const isDarkMode = () => typeof window !== "undefined" ? window?.matchMedia?.("(prefers-color-scheme:dark)").matches ?? false : false;
export {
  isDarkMode
};
