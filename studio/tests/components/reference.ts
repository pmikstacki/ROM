import { mount } from "svelte";
import ReferenceHarness from "./ReferenceHarness.svelte";
import "../../src/app.css";
mount(ReferenceHarness, { target: document.getElementById("app")! });
