import { createRoot, type Root } from "react-dom/client";
import { unstableSetRender } from "antd/es/config-provider/UnstableContext";

type RootContainer = Element & {
  __dmsxReactRoot?: Root;
};

unstableSetRender((node, container) => {
  const rootContainer = container as RootContainer;
  rootContainer.__dmsxReactRoot ??= createRoot(container);
  const root = rootContainer.__dmsxReactRoot;

  root.render(node);

  return async () => {
    await new Promise((resolve) => {
      setTimeout(resolve, 0);
    });
    root.unmount();
    delete rootContainer.__dmsxReactRoot;
  };
});
