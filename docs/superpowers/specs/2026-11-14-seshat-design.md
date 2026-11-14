# Seshat: de Kindle a EPUB en una sola app

Fecha: 2026-11-14. Estado: aprobado por el usuario (dirección e interfaz).

## Intención

Una app de escritorio para Windows, escrita en Rust, que convierte a EPUB los
libros Kindle comprados por el usuario sin que tenga que buscar herramientas,
comprobar versiones ni tocar carpetas. El usuario abre la app, ve sus libros,
elige una carpeta de destino y pulsa un botón.

## Objetivos

- Detectar sola la app Kindle de Microsoft Store, su versión y sus libros.
- Antes de cada desbloqueo, consultar la última publicación de
  `Satsuoni/DeDRM_tools`, descargarla si hace falta y verificar su huella.
- Bloquear el desbloqueo cuando la herramienta no admite la versión de Kindle
  instalada, y explicarlo.
- Mostrar el estado de cada libro: con DRM, desbloqueado, convertido.
- Convertir KFX sin DRM a EPUB 3 con código propio en Rust.
- Un solo `.exe`, arranque inmediato.

## Fuera de alcance

- Escribir código de descifrado propio. El desbloqueo lo hace el archiver de
  Satsuoni, que la app descarga y ejecuta como proceso externo.
- Cómics, maquetación fija, Print Replica, diccionarios.
- MOBI/AZW3 clásicos, macOS, Linux.
- Gestión de biblioteca (editar metadatos, enviar a dispositivos).

## Lo que haría fracasar el proyecto

- Ejecutar el desbloqueo con versiones incompatibles o sin poder determinarlo.
- Ejecutar un binario descargado cuya huella no coincide.
- Cerrar la app Kindle sin confirmación del usuario.
- Tocar o borrar los archivos originales de la carpeta de Kindle.

## Hechos verificados en el equipo del usuario (2026-11-14)

- Paquete: `AMZNKindle.AmazonKindleReadingApp`, versión `1.1.5634.0`,
  familia `AMZNKindle.AmazonKindleReadingApp_m1sc522ngdk36`. Proceso: `Kindle`.
- Contenido: `%LOCALAPPDATA%\Packages\<familia>\LocalState\Classic\Content\`,
  con `book_asset.db` y una carpeta `<ASIN>_EBOK` por libro. Dentro:
  `<ASIN>_EBOK.azw` (KFX cifrado, cabecera `\xeaDRMION\xee`), un
  `amzn1.drm-voucher.v1.<uuid>.voucher`, un `.phl`, un `.azw.md` y varios
  `.azw.res`.
- Libro de prueba: `B0GRCGCPMZ`.
- API: `GET https://api.github.com/repos/Satsuoni/DeDRM_tools/releases`.
  Todas las publicaciones llevan `prerelease: true`, así que `/releases/latest`
  no sirve: se toma la primera de la lista. Cada asset trae `digest`
  (`sha256:...`).
- `v10.0.35` (2026-11-08): `DeDRM_tools.zip`, sha256
  `5c85cce954b17836c2b421d6ae8b313f0aaeb190a9a3f81881636dbc80e25bc3`. Contiene
  `MSIXKFXArchiver_x64_1_5634.exe` y `MSIXKFXArchiver1_23620_final.exe`.
  Nota de la publicación: "Support for 1.5634 MSIX Kindle added".
- Invocación conocida del archiver (según `jaltez/kindle2epub`, con una versión
  anterior): `archiver.exe <carpeta_contenido> <carpeta_salida> <ruta.k4i>`.
  Produce un `.kfx-zip` sin DRM por libro. **Sin verificar para la versión
  x64**: hay que confirmarlo en el código fuente del repositorio de Satsuoni
  antes de implementar el paso de desbloqueo.

## Enfoques considerados

1. **Fork de `kindle2epub` (Python).** Rápido, pero el usuario lo descartó:
   quiere herramienta propia y rápida.
2. **Rust con descifrado propio.** Inviable: las claves están en la app Kindle,
   protegidas por el TPM. Además, es código de elusión que no se va a escribir.
3. **Rust que orquesta el archiver de Satsuoni y convierte con código propio.**
   Elegido. Automatiza todo lo que el usuario pidió y mantiene la parte
   delicada en la herramienta que ya la resuelve.

## Decisiones del usuario

- Lenguaje Rust, proyecto desde cero, en `C:\Users\jesus\Documents\Projects\`.
- Interfaz gráfica con las tres pantallas aprobadas: biblioteca, bloqueado,
  progreso. Maqueta: https://claude.ai/artifact/1E1kyUPYkngS6LB4Wu9vgF
- Descarga automática de la herramienta, comprobación de última versión antes
  de cada desbloqueo, bloqueo si no es compatible.
- Detección automática de libros y de cuáles están desbloqueados; carpeta de
  destino elegible.

## Decisiones del autor (corregibles)

- Nombre `seshat`. Licencia GPL-3.0-or-later (compatible con las fuentes de
  conocimiento del formato KFX).
- GUI con `eframe`/`egui`. Tema oscuro según la maqueta.
- Workspace de Cargo con tres crates:
  - `seshat-kfx`: lector de Ion binario, contenedor KFX, `.kfx-zip`, y
    generador de EPUB 3. Sin dependencias de Windows. Con tests.
  - `seshat-core`: detección de Kindle, publicaciones de GitHub,
    compatibilidad, descarga verificada, ejecución del archiver, estado de la
    biblioteca.
  - `seshat` (binario): interfaz `egui`. También acepta
    `seshat convert <archivo> -o <carpeta>` para usar el conversor sin GUI.
- Regla de compatibilidad, deducida del nombre del ejecutable en el zip:
  - `MSIXKFXArchiver_x64_1_<N>.exe` admite Kindle `1.1.<M>` con `M <= N`.
  - `MSIXKFXArchiver1_<N>_final.exe` admite Kindle `1.0.<M>` con `M <= N`.
  - Cualquier otro caso, o un nombre que no se pueda interpretar: bloqueado.
- Integridad: el zip se verifica contra el `digest` que da la API de GitHub.
  Sin `digest`, no se ejecuta.
- Datos de la app en `%LOCALAPPDATA%\seshat\`: herramienta en caché, copias
  desbloqueadas (`unlocked\<ASIN>.kfx-zip`), keyfile `.k4i`, ajustes y registro.
  Nunca se escribe en la carpeta de Kindle.
- Estado de un libro: "convertido" si existe el EPUB registrado en los ajustes;
  "desbloqueado" si existe su `.kfx-zip`; si no, "con DRM".
- Versión de Kindle: se obtiene con `Get-AppxPackage` vía PowerShell.
- La app pide confirmación antes de cerrar Kindle.
- Título del libro: se muestra el ASIN hasta que el libro esté desbloqueado; si
  el `.azw.md` resulta legible sin DRM, se usa para el título desde el inicio.

## Conversor KFX a EPUB (alcance inicial)

Novelas de texto reflowable: capítulos, párrafos, encabezados, énfasis, enlaces
internos, imágenes, portada, tabla de contenidos, metadatos (título, autor,
idioma, ASIN) y estilos básicos como CSS. Lo no reconocido se omite con aviso
en el registro; nunca debe abortar la conversión de un libro entero.

## Verificación

- Tests unitarios del lector Ion y del contenedor con datos construidos a mano.
- Prueba real: desbloquear `B0GRCGCPMZ`, convertirlo y validar el resultado con
  `epubcheck`. Comparar capítulos y recuento de palabras con la conversión de
  Calibre + KFX Input si está disponible.

## Queda para quien implemente

- Confirmar argumentos y salida del archiver x64.
- Fuente de la tabla de símbolos compartida `YJ_symbols` necesaria para leer KFX.
- Detalle del mapeo de estilos KFX a CSS.
