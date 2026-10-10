import { mount } from "svelte";
import SelectionLayoutCompositionHarness from "./SelectionLayoutCompositionHarness.svelte";
import "../../src/app.css";
mount(SelectionLayoutCompositionHarness, {
  target: document.getElementById("app")!,
});
