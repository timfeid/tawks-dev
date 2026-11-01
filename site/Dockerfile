# The developer docs site, served as static files. Build from the repository root:
#   docker build -f site/Dockerfile -t tawks-docs .
#   docker run -p 8790:80 tawks-docs
FROM node:24-bookworm-slim AS build
WORKDIR /src
RUN corepack enable && corepack prepare pnpm@10.20.0 --activate
COPY . .
RUN --mount=type=cache,id=pnpm-store,target=/pnpm/store \
  pnpm config set store-dir /pnpm/store && \
  pnpm install --frozen-lockfile
RUN pnpm --filter @tawks/site build && node site/scripts/add-examples.mjs

FROM nginx:1.27-alpine
COPY site/nginx.conf /etc/nginx/conf.d/default.conf
COPY --from=build /src/site/build /usr/share/nginx/html
EXPOSE 80
