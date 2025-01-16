#!/usr/bin/env bash

# Credits
# https://github.com/Spearfoot/FreeNAS-scripts/blob/master/get_hdd_temp.sh

fancontroller=/dev/ttyACM0
smartctl=/usr/local/sbin/smartctl

get_smart_drives()
{
  gs_smartdrives=""
  gs_drives=$("$smartctl" --scan | awk '{print $1}')

  for gs_drive in $gs_drives; do
    gs_smart_flag=$("$smartctl" -i "$gs_drive" | egrep "SMART support is:[[:blank:]]+Enabled" | awk '{print $4}')
    if [ "$gs_smart_flag" = "Enabled" ]; then
      gs_smartdrives="$gs_smartdrives $gs_drive"
    fi
  done
  echo "$gs_smartdrives"
}

print_cpu_temp()
{
  cpucores=$(sysctl -n hw.ncpu)

  printf '=== CPU (%s) ===\n' "$cpucores"

  cpucores=$((cpucores - 1))
  for core in $(seq 0 $cpucores); do
    temp=$(sysctl -n dev.cpu."$core".temperature | sed 's/\..*$//g')
    if [ "$temp" -lt 0 ]; then
      temp="--n/a--"
    else
      temp="${temp}C"
    fi
    printf 'CPU %2.2s: %5s\n' "$core" "$temp"
  done
  echo ""
}

print_cpu_temp

echo "=== DRIVES ==="

drives=$(get_smart_drives)
max_temp=0

for drive in $drives; do
  serial=$("$smartctl" -i "$drive" | grep -i "serial number" | awk '{print $NF}')
  capacity=$("$smartctl" -i "$drive" | grep "User Capacity" | awk '{print $5 $6}')

  temp=$("$smartctl" -A "$drive" | grep "194 Temperature" | awk '{print $10}')
  if [ -z "$temp" ]; then
    temp=$("$smartctl" -A "$drive" | grep "190 Temperature_Case" | awk '{print $10}')
  fi
  if [ -z "$temp" ]; then
    temp=$("$smartctl" -A "$drive" | grep "190 Airflow_Temperature" | awk '{print $10}')
  fi
  if [ -z "$temp" ]; then
    temp=$("$smartctl" -A "$drive" | grep "Current Drive Temperature" | awk '{print $4}')
  fi

  if [ -z "$temp" ]; then
    temp="-n/a-"
  else
    temp="${temp}C"
    if [ "$temp" -gt "$max_temp" ]; then
      max_temp=$temp
    fi
  fi

  dfamily=$("$smartctl" -i "$drive" | grep "Model Family" | awk '{print $3, $4, $5, $6, $7}' | sed -e 's/[[:space:]]*$//')
  dmodel=$("$smartctl" -i "$drive" | grep "Device Model" | awk '{print $3, $4, $5, $6, $7}' | sed -e 's/[[:space:]]*$//')

  if [ -z "$dfamily" ]; then
    dinfo="$dmodel"
  else
    dinfo="$dfamily ($dmodel)"
  fi

  if [ -z "$dfamily" ]; then
    vendor=$("$smartctl" -i "$drive" | grep "Vendor:" | awk '{print $NF}')
    product=$("$smartctl" -i "$drive" | grep "Product:" | awk '{print $NF}')
    revision=$("$smartctl" -i "$drive" | grep "Revision:" | awk '{print $NF}')
    dinfo="$vendor $product $revision"
  fi

  printf '%6.6s: %5s %-8s %-20.20s %s\n' "$(basename "$drive")" "$temp" "$capacity" "$serial" "$dinfo"
done

speed="0" # max 256
if [ "$max_temp" -gt "30" ]; then
  speed="100"
fi
if [ "$max_temp" -gt "35" ]; then
  speed="200"
fi
if [ "$max_temp" -gt "40" ]; then
  speed="256"
fi

echo "$speed" > "$fancontroller"
# cat < "$fancontroller"
