import { mount } from "svelte";
import "@fontsource-variable/inter/wght.css";
import "@fontsource-variable/jetbrains-mono/wght.css";
import "./style.css";
import App from "./App.svelte";

mount(App, { target: document.getElementById("app")! });
