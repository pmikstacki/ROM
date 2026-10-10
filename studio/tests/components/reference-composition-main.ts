import { mount } from "svelte";
import ReferenceCompositionHarness from "./ReferenceCompositionHarness.svelte";
import "../../src/app.css";
mount(ReferenceCompositionHarness, { target: document.getElementById("app")! });
