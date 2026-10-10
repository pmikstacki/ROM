import "../../src/app.css";
import {mount} from "svelte";
import Harness from "./shared-primitive-identity.svelte";
mount(Harness,{target:document.getElementById("app")!});
