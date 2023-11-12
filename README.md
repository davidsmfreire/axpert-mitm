# Axpert with a Man in the middle

This is a personal project featuring a TCP interceptor that captures data from a Voltronix Axpert inverter to store in MySQL database and integrate with Home Assistant via RESTApi.

This was possible because I'm using a Raspberry PI as the wireless access point for the inverter, therefore gaining access to all packet traffic between the inverter and the Hong Kong data center gathering all my precious energy information >:(.

Although the code is highly personal, if you still want to use this, you need to host a MySQL database and optionally have a Home Assistant instance as well.
Beware, the code IS the documentation, good luck!
