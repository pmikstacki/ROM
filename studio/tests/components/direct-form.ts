import { mount } from "svelte";
import DirectFormHarness from "./DirectFormHarness.svelte";
import "../../src/app.css";
mount(DirectFormHarness, { target: document.getElementById("app")! });
