const isFirefox = () => typeof window !== "undefined" && !!window["InstallTrigger"];
export {
  isFirefox
};
