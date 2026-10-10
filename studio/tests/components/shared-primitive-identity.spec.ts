import {test,expect} from "@playwright/test";
test("Studio facades use the shared installed control implementations",async({page})=>{
  await page.goto("/rom-studio/tests/components/shared-primitive-identity.html");
  const output=page.getByTestId("shared-identity");
  await expect(output).toBeVisible();
  const identity=JSON.parse(await output.textContent()??"{}");
  expect(Object.keys(identity)).toHaveLength(16);
  expect(Object.entries(identity).filter(([,shared])=>shared!==true)).toEqual([]);
});
