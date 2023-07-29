import { OAuthClient } from "@timetreeapp/web-api";

const client = new OAuthClient(process.env.TIMETREE_ACCESS_TOKEN!);

(async () => {
  const data = await client.getCalendars();
  console.log("calendars", data);
})();
