import { mount } from "svelte";
import App from "./App.svelte";
import {
  createStudioBootstrap,
  parseStudioBootstrap,
} from "./lib/application/bootstrap.ts";
import "./app.css";

async function start() {
  const target = document.getElementById("app");
  let bootstrap: Awaited<ReturnType<typeof createStudioBootstrap>> | undefined;
  try {
    if (!target) throw Error("Studio mount target is absent.");
    const profiles = document.querySelectorAll("#rom-studio-auth-profile");
    const element = profiles[0];
    if (
      profiles.length !== 1 ||
      element.tagName !== "SCRIPT" ||
      element.getAttribute("type") !== "application/json"
    )
      throw Error("Studio host configuration is absent or ambiguous.");
    bootstrap = await createStudioBootstrap(
      parseStudioBootstrap(element.textContent ?? ""),
    );
    mount(App, { target, props: { authProfile: bootstrap.profile } });
    window.addEventListener("beforeunload", bootstrap.close, { once: true });
  } catch {
    bootstrap?.close();
    const errorTarget = target ?? document.body;
    errorTarget.replaceChildren();
    const message = document.createElement("p");
    message.setAttribute("role", "alert");
    message.textContent =
      "Studio could not start. Check host configuration and browser storage.";
    errorTarget.append(message);
  }
}
void start();
