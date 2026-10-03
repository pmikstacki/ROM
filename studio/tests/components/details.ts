import { mount } from "svelte";
import DetailsHarness from "./DetailsHarness.svelte";
import "../../src/app.css";
mount(DetailsHarness, { target: document.getElementById("app")! });
