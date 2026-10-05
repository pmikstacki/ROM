import { mount } from "svelte";
import { App, registerRenderer } from "rom-studio";
import "rom-studio/styles";
import AuthorCode from "./AuthorCode.svelte";

// Extensions use codec identity. Resource names never select a renderer.
registerRenderer({ name: "demo-ticket-code", version: 1 }, AuthorCode, {
  layout: "inline",
});
mount(App, { target: document.getElementById("app")! });
