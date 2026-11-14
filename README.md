# Anzu

De Kindle a EPUB en una sola app para Windows, escrita en Rust.

Anzu encuentra los libros que descargaste con la app Kindle, comprueba que la
herramienta de desbloqueo es la última y admite tu versión de Kindle, la ejecuta y
convierte el resultado a EPUB. No hay que buscar descargas ni vigilar versiones.

## Requisitos

- Windows 10 u 11.
- La app **Amazon Kindle** de Microsoft Store, con tu sesión iniciada y los libros
  descargados.
- Conexión a internet para comprobar la herramienta antes de cada desbloqueo.

## Uso

Abre `anzu.exe`. La ventana muestra:

- **Estado del sistema:** versión de Kindle instalada, versión de la herramienta y si
  son compatibles.
- **Tus libros:** con DRM, desbloqueados o ya convertidos.
- **Carpeta de destino** y el botón **Desbloquear y convertir**.

Si la app Kindle está abierta, Anzu pide permiso antes de cerrarla.

También convierte desde la terminal un libro ya desbloqueado:

```
anzu convert libro.kfx-zip -o C:\Libros
```

## Qué hace y qué no

| Parte | Quién la hace |
|---|---|
| Detectar Kindle y los libros | Anzu |
| Comprobar la última versión y la compatibilidad | Anzu |
| Descargar la herramienta y verificar su huella SHA-256 | Anzu |
| Quitar el DRM | La herramienta de [Satsuoni](https://github.com/Satsuoni/DeDRM_tools), como proceso externo |
| Leer el formato KFX y escribir el EPUB | Anzu, con código propio |

Anzu no contiene código de descifrado.

### Cuándo se bloquea el desbloqueo

Anzu se niega a desbloquear, y explica por qué, cuando:

- tu app Kindle es más nueva que la última herramienta publicada;
- no puede determinar con certeza si son compatibles;
- no hay conexión para comprobarlo;
- la descarga no trae huella SHA-256, o la huella no coincide.

Los libros ya desbloqueados se pueden convertir siempre.

### Alcance del conversor

Libros de texto: capítulos, encabezados, cursiva y negrita, enlaces internos y
externos, imágenes, portada, índice y metadatos. Quedan fuera los cómics, la
maquetación fija y los diccionarios. Lo que no se reconoce se omite con un aviso; no
interrumpe la conversión.

## Dónde guarda cosas

Todo en `%LOCALAPPDATA%\anzu\`:

| Ruta | Contenido |
|---|---|
| `tool\<versión>\` | La herramienta descargada |
| `unlocked\<ASIN>.kfx-zip` | Copias desbloqueadas de tus libros |
| `keys\anzu.k4i`, `work\books.keyfile` | **Secretos de tu cuenta de Amazon** |
| `settings.json` | Carpeta de destino y libros convertidos |

Los dos archivos de claves equivalen a una contraseña: no los compartas ni los subas
a ningún sitio. La herramienta también imprime esos secretos al trabajar; Anzu los
filtra antes de mostrarlos o copiarlos en el registro.

Anzu nunca escribe en la carpeta de la app Kindle.

## Compilar

```
cargo build --release
```

El ejecutable queda en `target\release\anzu.exe`.

```
cargo test --workspace
```

## Estructura

| Crate | Responsabilidad |
|---|---|
| `anzu-kfx` | Lector de Ion binario y del contenedor KFX; modelo del libro; escritor de EPUB 3 |
| `anzu-core` | Detección de Kindle, publicaciones de GitHub, compatibilidad, descarga verificada, ejecución de la herramienta, biblioteca |
| `anzu` | Interfaz (`egui`) y línea de comandos |

## Créditos

- **Satsuoni**, por la herramienta de desbloqueo.
- **John Howell** (jhowell), cuyo plugin KFX Input es la referencia del formato KFX.

## Licencia y aviso legal

GPL-3.0-or-later. Anzu no está aprobada por Amazon.

Quitar el DRM puede estar restringido donde vives. Úsala solo con libros que hayas
comprado y para leerlos en tus propios dispositivos.
